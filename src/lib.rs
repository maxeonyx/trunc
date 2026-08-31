use regex::Regex;
use std::collections::VecDeque;
use std::io::{self, BufRead, Write};

#[cfg(unix)]
use std::mem;
#[cfg(unix)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Debug)]
pub struct Config {
    pub first: usize,
    pub last: usize,
    pub match_first: usize,
    pub match_last: usize,
    pub context: usize,
    pub width: usize,
    pub pattern: Option<String>,
}

#[derive(Debug)]
pub enum RunError {
    InvalidPattern(regex::Error),
    Read(io::Error),
    Write(io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Completed,
    BrokenPipe,
    Interrupted(InterruptSignal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FinishReason {
    Completed,
    Interrupted(InterruptSignal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptSignal {
    SigInt,
    SigTerm,
}

impl InterruptSignal {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::SigInt => 130,
            Self::SigTerm => 143,
        }
    }

    #[cfg(unix)]
    fn from_raw(signal: usize) -> Option<Self> {
        match signal as i32 {
            libc::SIGINT => Some(Self::SigInt),
            libc::SIGTERM => Some(Self::SigTerm),
            _ => None,
        }
    }
}

pub fn run<R: BufRead, W: Write>(
    mut reader: R,
    writer: W,
    config: Config,
) -> Result<RunOutcome, RunError> {
    let mut truncator = Truncator::new(config).map_err(RunError::InvalidPattern)?;
    let mut output = Output::new(writer);
    let mut line_buffer = Vec::new();

    #[cfg(unix)]
    let _pending_interrupt_guard = PendingInterruptGuard::install();

    loop {
        let line =
            match read_lossy_line(&mut reader, &mut line_buffer, &mut truncator, &mut output)? {
                ReadLine::Line(line) => line,
                ReadLine::Eof => break,
                ReadLine::Retry => continue,
            };

        match truncator.process_line(&line, &mut output) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                return Ok(RunOutcome::BrokenPipe);
            }
            Err(error) => return Err(RunError::Write(error)),
        }

        if let Some(result) = finish_if_interrupted(&mut truncator, &mut output) {
            return result;
        }
    }

    if let Some(result) = finish_if_interrupted(&mut truncator, &mut output) {
        return result;
    }

    finish_with_reason(&mut truncator, &mut output, FinishReason::Completed)
}

enum ReadLine {
    Line(String),
    Eof,
    Retry,
}

fn read_lossy_line<R: BufRead, W: Write>(
    reader: &mut R,
    line_buffer: &mut Vec<u8>,
    truncator: &mut Truncator,
    output: &mut Output<W>,
) -> Result<ReadLine, RunError> {
    line_buffer.clear();

    let bytes_read = match reader.read_until(b'\n', line_buffer) {
        Ok(bytes_read) => bytes_read,
        Err(error) => {
            if let Some(result) = finish_if_interrupted(truncator, output) {
                return result.map(|_| ReadLine::Eof);
            }

            if error.kind() == io::ErrorKind::Interrupted {
                return Ok(ReadLine::Retry);
            }

            return Err(RunError::Read(error));
        }
    };

    if bytes_read == 0 {
        return Ok(ReadLine::Eof);
    }

    strip_trailing_newline_bytes(line_buffer);
    Ok(ReadLine::Line(
        String::from_utf8_lossy(line_buffer).into_owned(),
    ))
}

fn finish_if_interrupted<W: Write>(
    truncator: &mut Truncator,
    output: &mut Output<W>,
) -> Option<Result<RunOutcome, RunError>> {
    pending_interrupt_signal()
        .map(|signal| finish_with_reason(truncator, output, FinishReason::Interrupted(signal)))
}

fn finish_with_reason<W: Write>(
    truncator: &mut Truncator,
    output: &mut Output<W>,
    reason: FinishReason,
) -> Result<RunOutcome, RunError> {
    match truncator.finish(output, reason) {
        Ok(()) => Ok(match reason {
            FinishReason::Completed => RunOutcome::Completed,
            FinishReason::Interrupted(signal) => RunOutcome::Interrupted(signal),
        }),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(RunOutcome::BrokenPipe),
        Err(error) => Err(RunError::Write(error)),
    }
}

fn strip_trailing_newline_bytes(line: &mut Vec<u8>) {
    if line.last() == Some(&b'\n') {
        line.pop();

        if line.last() == Some(&b'\r') {
            line.pop();
        }
    }
}

fn interrupted_marker(
    lines_truncated: usize,
    remaining_matches: usize,
    total_matches: usize,
) -> String {
    if remaining_matches > 0 {
        format!(
            "[... {} lines and {} matches truncated ({} total), interrupted ...]",
            lines_truncated, remaining_matches, total_matches
        )
    } else {
        format!("[... {} lines truncated, interrupted ...]", lines_truncated)
    }
}

#[cfg(unix)]
fn pending_interrupt_signal() -> Option<InterruptSignal> {
    INTERRUPT_STATE.load()
}

#[cfg(not(unix))]
fn pending_interrupt_signal() -> Option<InterruptSignal> {
    None
}

struct Output<W> {
    writer: W,
}

impl<W: Write> Output<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }

    fn write_line(&mut self, line: &str) -> io::Result<()> {
        writeln!(self.writer, "{line}")
    }

    fn write_line_and_flush(&mut self, line: &str) -> io::Result<()> {
        self.write_line(line)?;
        self.flush()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

struct Truncator {
    first_count: usize,
    last_count: usize,
    context_size: usize,
    match_first_count: usize,
    match_last_count: usize,
    width: usize,
    pattern: Option<Regex>,
    line_number: usize,
    head_output_count: usize,
    streamed_match_count: usize,
    total_matches: usize,
    last_output_line: usize,
    match_output_ranges: Vec<(usize, usize)>,
    tail_buffer: VecDeque<(usize, String)>,
    context_buffer: VecDeque<(usize, String)>,
    after_context_remaining: usize,
    deferred_match_groups: VecDeque<MatchGroup>,
}

#[derive(Clone)]
struct MatchGroup {
    match_number: usize,
    start_line: usize,
    end_line: usize,
    lines: Vec<(usize, String)>,
}

struct PatternFinishState {
    tail_groups: Vec<MatchGroup>,
    hidden_transition_matches: usize,
    hidden_final_gap_matches: usize,
}

impl MatchGroup {
    fn new(
        match_number: usize,
        match_line: usize,
        context_size: usize,
        leading_context: &VecDeque<(usize, String)>,
        match_content: &str,
    ) -> Self {
        let start_line = match_line.saturating_sub(context_size);
        let end_line = match_line.saturating_add(context_size);

        let mut lines = Vec::new();
        for (line_number, content) in leading_context {
            if *line_number >= start_line && *line_number < match_line {
                lines.push((*line_number, content.clone()));
            }
        }
        lines.push((match_line, match_content.to_string()));

        Self {
            match_number,
            start_line,
            end_line,
            lines,
        }
    }

    fn push_line_if_in_range(&mut self, line_number: usize, content: &str) {
        if line_number > self.start_line
            && line_number <= self.end_line
            && self.lines.last().map(|(number, _)| *number) != Some(line_number)
        {
            self.lines.push((line_number, content.to_string()));
        }
    }
}

impl Truncator {
    fn new(config: Config) -> Result<Self, regex::Error> {
        let pattern = match config.pattern {
            Some(pattern) => Some(Regex::new(&pattern)?),
            None => None,
        };

        Ok(Self {
            first_count: config.first,
            last_count: config.last,
            context_size: config.context,
            match_first_count: config.match_first,
            match_last_count: config.match_last,
            width: config.width,
            pattern,
            line_number: 0,
            head_output_count: 0,
            streamed_match_count: 0,
            total_matches: 0,
            last_output_line: 0,
            match_output_ranges: Vec::new(),
            tail_buffer: VecDeque::new(),
            context_buffer: VecDeque::new(),
            after_context_remaining: 0,
            deferred_match_groups: VecDeque::new(),
        })
    }

    fn process_line<W: Write>(&mut self, content: &str, output: &mut Output<W>) -> io::Result<()> {
        self.line_number += 1;

        if self.should_stream_head() {
            self.stream_head_line(output, content)?;
            return Ok(());
        }

        self.buffer_tail_line(content);

        if self.pattern.is_some() {
            self.process_pattern_line(output, content)?;
        }

        Ok(())
    }

    fn should_stream_head(&self) -> bool {
        self.head_output_count < self.first_count
    }

    fn stream_head_line<W: Write>(
        &mut self,
        output: &mut Output<W>,
        content: &str,
    ) -> io::Result<()> {
        output.write_line_and_flush(&truncate_line(content, self.width))?;
        self.head_output_count += 1;
        self.last_output_line = self.line_number;
        Ok(())
    }

    fn buffer_tail_line(&mut self, content: &str) {
        self.tail_buffer
            .push_back((self.line_number, content.to_string()));
        if self.tail_buffer.len() > self.last_count {
            self.tail_buffer.pop_front();
        }
    }

    fn process_pattern_line<W: Write>(
        &mut self,
        output: &mut Output<W>,
        content: &str,
    ) -> io::Result<()> {
        self.append_line_to_deferred_match_groups(content);

        if self.after_context_remaining > 0 {
            self.stream_after_context_line(output, content)?;
        }

        if self.is_match(content) {
            self.total_matches += 1;
            self.capture_match_group(content);

            if self.can_show_another_head_match() {
                self.show_streamed_match(output, content)?;
            }
        }

        self.buffer_context_line(content);
        Ok(())
    }

    fn stream_after_context_line<W: Write>(
        &mut self,
        output: &mut Output<W>,
        content: &str,
    ) -> io::Result<()> {
        if self.line_number > self.last_output_line {
            self.write_streamed_line(output, self.line_number, content)?;
        }
        self.after_context_remaining -= 1;
        Ok(())
    }

    fn is_match(&self, content: &str) -> bool {
        self.pattern
            .as_ref()
            .is_some_and(|regex| regex.is_match(content))
    }

    fn can_show_another_head_match(&self) -> bool {
        self.streamed_match_count < self.match_first_count
    }

    fn show_streamed_match<W: Write>(
        &mut self,
        output: &mut Output<W>,
        content: &str,
    ) -> io::Result<()> {
        let match_number = self.total_matches;
        self.streamed_match_count += 1;

        let lines_truncated =
            self.lines_truncated_before_line(self.line_number.saturating_sub(self.context_size));
        self.write_match_marker(output, lines_truncated, match_number, 0)?;
        self.replay_pending_context(output)?;

        if self.line_number > self.last_output_line {
            self.write_streamed_line(output, self.line_number, content)?;
        }

        self.after_context_remaining = self.context_size;
        Ok(())
    }

    fn write_match_marker<W: Write>(
        &mut self,
        output: &mut Output<W>,
        lines_truncated: usize,
        match_number: usize,
        hidden_matches: usize,
    ) -> io::Result<()> {
        if lines_truncated > 0 || self.should_show_zero_gap_match_marker(match_number) {
            let marker = if hidden_matches > 0 {
                format!(
                    "[... {} lines and {} matches truncated, match {} shown ...]",
                    lines_truncated, hidden_matches, match_number
                )
            } else {
                format!(
                    "[... {} lines truncated, match {} shown ...]",
                    lines_truncated, match_number
                )
            };
            output.write_line_and_flush(&marker)?;
        }

        Ok(())
    }

    fn lines_truncated_before_line(&self, line_number: usize) -> usize {
        let context_start = line_number;
        let gap_start = self.last_output_line.saturating_add(1);
        let gap_end = context_start.max(gap_start);
        gap_end.saturating_sub(gap_start)
    }

    fn should_show_zero_gap_match_marker(&self, match_number: usize) -> bool {
        match_number == 1 && self.last_output_line >= self.first_count
    }

    fn capture_match_group(&mut self, content: &str) {
        if self.match_last_count == 0 {
            return;
        }

        self.deferred_match_groups.push_back(MatchGroup::new(
            self.total_matches,
            self.line_number,
            self.context_size,
            &self.context_buffer,
            content,
        ));

        while self.deferred_match_groups.len() > self.match_last_count {
            self.deferred_match_groups.pop_front();
        }
    }

    fn append_line_to_deferred_match_groups(&mut self, content: &str) {
        for group in &mut self.deferred_match_groups {
            group.push_line_if_in_range(self.line_number, content);
        }
    }

    fn replay_pending_context<W: Write>(&mut self, output: &mut Output<W>) -> io::Result<()> {
        let pending_context: Vec<(usize, String)> = self.context_buffer.iter().cloned().collect();
        for (context_line_number, context_content) in pending_context {
            if context_line_number > self.last_output_line && context_line_number < self.line_number
            {
                self.write_streamed_line(output, context_line_number, &context_content)?;
            }
        }
        Ok(())
    }

    fn buffer_context_line(&mut self, content: &str) {
        self.context_buffer
            .push_back((self.line_number, content.to_string()));
        if self.context_buffer.len() > self.context_size {
            self.context_buffer.pop_front();
        }
    }

    fn finish<W: Write>(&mut self, output: &mut Output<W>, reason: FinishReason) -> io::Result<()> {
        let total_lines = self.line_number;
        if total_lines == 0 {
            return output.flush();
        }

        let tail_start = if total_lines > self.last_count {
            (total_lines - self.last_count).saturating_add(1)
        } else {
            1
        };
        let needs_truncation = total_lines > self.first_count.saturating_add(self.last_count);

        if self.pattern.is_some() {
            self.finish_pattern_output(output, reason, tail_start, needs_truncation)?;
        } else if let Some(marker) = self.final_default_marker(reason, needs_truncation) {
            output.write_line(&marker)?;
        }

        for (tail_line_number, tail_content) in &self.tail_buffer {
            if *tail_line_number > self.first_count && !self.was_output_in_match(*tail_line_number)
            {
                output.write_line(&truncate_line(tail_content, self.width))?;
            }
        }

        output.flush()
    }

    fn finish_pattern_output<W: Write>(
        &mut self,
        output: &mut Output<W>,
        reason: FinishReason,
        tail_start: usize,
        needs_truncation: bool,
    ) -> io::Result<()> {
        let finish_state = self.pattern_finish_state();

        if let Some(marker) = self.transition_marker(reason, &finish_state) {
            output.write_line(&marker)?;
        }

        self.write_deferred_match_groups(output, &finish_state.tail_groups)?;

        if let Some(marker) =
            self.final_pattern_tail_marker(reason, tail_start, needs_truncation, &finish_state)
        {
            output.write_line(&marker)?;
        }

        Ok(())
    }

    fn pattern_finish_state(&self) -> PatternFinishState {
        let tail_groups: Vec<MatchGroup> = self
            .deferred_match_groups
            .iter()
            .filter(|group| group.match_number > self.streamed_match_count)
            .cloned()
            .collect();
        let hidden_transition_matches = self
            .total_matches
            .saturating_sub(self.streamed_match_count.saturating_add(tail_groups.len()));
        let hidden_final_gap_matches = if tail_groups.is_empty() {
            self.total_matches.saturating_sub(self.streamed_match_count)
        } else {
            0
        };

        PatternFinishState {
            tail_groups,
            hidden_transition_matches,
            hidden_final_gap_matches,
        }
    }

    fn final_pattern_tail_marker(
        &self,
        reason: FinishReason,
        tail_start: usize,
        needs_truncation: bool,
        finish_state: &PatternFinishState,
    ) -> Option<String> {
        if self.total_matches > 0 {
            return self.final_pattern_marker_with_matches(reason, tail_start, finish_state);
        }

        if !needs_truncation {
            return None;
        }

        let lines_truncated = self.line_number - self.first_count - self.last_count;
        Some(match reason {
            FinishReason::Interrupted(_) => {
                interrupted_marker(lines_truncated, 0, self.total_matches)
            }
            FinishReason::Completed => {
                format!(
                    "[... {} lines truncated, 0 matches found ...]",
                    lines_truncated
                )
            }
        })
    }

    fn final_pattern_marker_with_matches(
        &self,
        reason: FinishReason,
        tail_start: usize,
        finish_state: &PatternFinishState,
    ) -> Option<String> {
        let lines_truncated = self.lines_truncated_before_tail(tail_start);
        let remaining_matches = finish_state.hidden_final_gap_matches;

        if lines_truncated == 0 && remaining_matches == 0 {
            return None;
        }

        Some(match reason {
            FinishReason::Interrupted(_) => {
                interrupted_marker(lines_truncated, remaining_matches, self.total_matches)
            }
            FinishReason::Completed if remaining_matches > 0 => format!(
                "[... {} lines and {} matches truncated ({} total) ...]",
                lines_truncated, remaining_matches, self.total_matches
            ),
            FinishReason::Completed => format!("[... {} lines truncated ...]", lines_truncated),
        })
    }

    fn lines_truncated_before_tail(&self, tail_start: usize) -> usize {
        let gap_start = self.last_output_line.saturating_add(1);
        let gap_end = tail_start;
        gap_end.saturating_sub(gap_start)
    }

    fn transition_marker(
        &self,
        reason: FinishReason,
        finish_state: &PatternFinishState,
    ) -> Option<String> {
        let first_tail_group = finish_state.tail_groups.first()?;
        if finish_state.hidden_transition_matches == 0 {
            return None;
        }
        let lines_truncated = self.lines_truncated_before_line(first_tail_group.start_line);

        Some(match reason {
            FinishReason::Interrupted(_) => format!(
                "[... {} lines and {} matches truncated, match {} shown, interrupted ...]",
                lines_truncated,
                finish_state.hidden_transition_matches,
                first_tail_group.match_number
            ),
            FinishReason::Completed => format!(
                "[... {} lines and {} matches truncated, match {} shown ...]",
                lines_truncated,
                finish_state.hidden_transition_matches,
                first_tail_group.match_number
            ),
        })
    }

    fn write_deferred_match_groups<W: Write>(
        &mut self,
        output: &mut Output<W>,
        tail_groups: &[MatchGroup],
    ) -> io::Result<()> {
        let has_transition_marker = !tail_groups.is_empty()
            && self
                .total_matches
                .saturating_sub(self.streamed_match_count.saturating_add(tail_groups.len()))
                > 0;

        for (index, group) in tail_groups.iter().enumerate() {
            let needs_marker = index > 0 || !has_transition_marker;

            if needs_marker {
                let lines_truncated = self.lines_truncated_before_line(group.start_line);
                if lines_truncated > 0 || self.should_show_zero_gap_match_marker(group.match_number)
                {
                    self.write_match_marker(output, lines_truncated, group.match_number, 0)?;
                }
            }

            for (line_number, content) in &group.lines {
                if *line_number > self.last_output_line && *line_number <= self.line_number {
                    self.write_streamed_line(output, *line_number, content)?;
                }
            }
        }

        Ok(())
    }

    fn final_default_marker(&self, reason: FinishReason, needs_truncation: bool) -> Option<String> {
        if !needs_truncation {
            return None;
        }

        let lines_truncated = self.line_number - self.first_count - self.last_count;
        Some(match reason {
            FinishReason::Interrupted(_) => {
                interrupted_marker(lines_truncated, 0, self.total_matches)
            }
            FinishReason::Completed => format!("[... {} lines truncated ...]", lines_truncated),
        })
    }

    fn write_streamed_line<W: Write>(
        &mut self,
        output: &mut Output<W>,
        line_number: usize,
        content: &str,
    ) -> io::Result<()> {
        output.write_line_and_flush(&truncate_line(content, self.width))?;
        self.record_match_output(line_number);
        self.last_output_line = line_number;
        Ok(())
    }

    fn record_match_output(&mut self, line_number: usize) {
        if let Some((_, end)) = self.match_output_ranges.last_mut() {
            if line_number == end.saturating_add(1) {
                *end = line_number;
                return;
            }
        }

        self.match_output_ranges.push((line_number, line_number));
    }

    fn was_output_in_match(&self, line_number: usize) -> bool {
        let range_index = self
            .match_output_ranges
            .partition_point(|(_, end)| *end < line_number);

        self.match_output_ranges
            .get(range_index)
            .is_some_and(|(start, end)| line_number >= *start && line_number <= *end)
    }
}

fn truncate_line(line: &str, width: usize) -> String {
    if width == 0 {
        return line.to_string();
    }

    let char_count = line.chars().count();
    let max_len = width.saturating_mul(2);

    if char_count <= max_len {
        return line.to_string();
    }

    let removed = char_count - max_len;
    let marker = format!("[... {} chars ...]", removed);

    let result_len = width.saturating_add(marker.len()).saturating_add(width);
    if result_len >= char_count {
        return line.to_string();
    }

    let first: String = line.chars().take(width).collect();
    let last: String = line.chars().skip(char_count - width).collect();
    format!("{}{}{}", first, marker, last)
}

#[cfg(unix)]
static INTERRUPT_STATE: InterruptState = InterruptState::new();

#[cfg(unix)]
struct InterruptState {
    pending_signal: AtomicUsize,
}

#[cfg(unix)]
impl InterruptState {
    const fn new() -> Self {
        Self {
            pending_signal: AtomicUsize::new(0),
        }
    }

    fn install(&self) -> PendingInterruptGuard {
        self.pending_signal.store(0, Ordering::SeqCst);

        install_interrupt_handler(libc::SIGINT);
        install_interrupt_handler(libc::SIGTERM);

        PendingInterruptGuard { active: true }
    }

    fn load(&self) -> Option<InterruptSignal> {
        InterruptSignal::from_raw(self.pending_signal.load(Ordering::SeqCst))
    }

    fn restore(&self) {
        self.pending_signal.store(0, Ordering::SeqCst);
    }
}

#[cfg(unix)]
fn install_interrupt_handler(signal: i32) {
    let mut action: libc::sigaction = unsafe { mem::zeroed() };
    action.sa_flags = 0;
    action.sa_sigaction = record_interrupt_signal as *const () as usize;

    unsafe {
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(signal, &action, std::ptr::null_mut());
    }
}

#[cfg(unix)]
extern "C" fn record_interrupt_signal(signal: i32) {
    INTERRUPT_STATE
        .pending_signal
        .store(signal as usize, Ordering::SeqCst);

    // Wake any blocking stdin read so the main loop can notice the pending
    // interrupt and flush buffered output before exiting.
    unsafe {
        libc::close(libc::STDIN_FILENO);
    }
}

#[cfg(unix)]
struct PendingInterruptGuard {
    active: bool,
}

#[cfg(unix)]
impl PendingInterruptGuard {
    fn install() -> Self {
        INTERRUPT_STATE.install()
    }
}

#[cfg(unix)]
impl Drop for PendingInterruptGuard {
    fn drop(&mut self) {
        if self.active {
            INTERRUPT_STATE.restore();
            self.active = false;
        }
    }
}
