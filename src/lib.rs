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
    pub matches: usize,
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
    let mut line = String::new();

    #[cfg(unix)]
    let _pending_interrupt_guard = PendingInterruptGuard::install();

    loop {
        line.clear();

        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                if let Some(result) = finish_if_interrupted(&mut truncator, &mut output) {
                    return result;
                }
                continue;
            }
            Err(error) => {
                if let Some(result) = finish_if_interrupted(&mut truncator, &mut output) {
                    return result;
                }
                return Err(RunError::Read(error));
            }
        }

        strip_trailing_newline(&mut line);

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

fn strip_trailing_newline(line: &mut String) {
    if line.ends_with('\n') {
        line.pop();

        if line.ends_with('\r') {
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
    max_matches: usize,
    width: usize,
    pattern: Option<Regex>,
    line_number: usize,
    head_output_count: usize,
    in_middle: bool,
    matches_shown: usize,
    total_matches: usize,
    last_output_line: usize,
    match_output_ranges: Vec<(usize, usize)>,
    tail_buffer: VecDeque<(usize, String)>,
    context_buffer: VecDeque<(usize, String)>,
    after_context_remaining: usize,
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
            max_matches: config.matches,
            width: config.width,
            pattern,
            line_number: 0,
            head_output_count: 0,
            in_middle: false,
            matches_shown: 0,
            total_matches: 0,
            last_output_line: 0,
            match_output_ranges: Vec::new(),
            tail_buffer: VecDeque::with_capacity(config.last + 1),
            context_buffer: VecDeque::with_capacity(config.context + 1),
            after_context_remaining: 0,
        })
    }

    fn process_line<W: Write>(&mut self, content: &str, output: &mut Output<W>) -> io::Result<()> {
        self.line_number += 1;

        if self.head_output_count < self.first_count {
            output.write_line_and_flush(&truncate_line(content, self.width))?;
            self.head_output_count += 1;
            self.last_output_line = self.line_number;
            return Ok(());
        }

        if !self.in_middle {
            self.in_middle = true;
        }

        self.tail_buffer
            .push_back((self.line_number, content.to_string()));
        if self.tail_buffer.len() > self.last_count {
            self.tail_buffer.pop_front();
        }

        let is_match = self
            .pattern
            .as_ref()
            .is_some_and(|regex| regex.is_match(content));

        if self.pattern.is_some() {
            if self.after_context_remaining > 0 {
                if self.line_number > self.last_output_line {
                    self.write_streamed_line(output, self.line_number, content)?;
                }
                self.after_context_remaining -= 1;
            }

            if is_match {
                self.total_matches += 1;

                if self.matches_shown < self.max_matches {
                    self.matches_shown += 1;

                    let context_start = self.line_number.saturating_sub(self.context_size);
                    let gap_start = self.last_output_line + 1;
                    let gap_end = context_start.max(gap_start);
                    let lines_truncated = gap_end.saturating_sub(gap_start);

                    let match_annotation = if self.matches_shown == self.max_matches {
                        format!("match {}/{}", self.matches_shown, self.max_matches)
                    } else {
                        format!("match {}", self.matches_shown)
                    };

                    if lines_truncated > 0 {
                        output.write_line_and_flush(&format!(
                            "[... {} lines truncated, {} shown ...]",
                            lines_truncated, match_annotation
                        ))?;
                    } else if self.matches_shown == 1 && self.last_output_line >= self.first_count {
                        output.write_line_and_flush(&format!(
                            "[... 0 lines truncated, {} shown ...]",
                            match_annotation
                        ))?;
                    }

                    let pending_context: Vec<(usize, String)> =
                        self.context_buffer.iter().cloned().collect();
                    for (context_line_number, context_content) in pending_context {
                        if context_line_number > self.last_output_line
                            && context_line_number < self.line_number
                        {
                            self.write_streamed_line(
                                output,
                                context_line_number,
                                &context_content,
                            )?;
                        }
                    }

                    if self.line_number > self.last_output_line {
                        self.write_streamed_line(output, self.line_number, content)?;
                    }

                    self.after_context_remaining = self.context_size;
                }
            }

            self.context_buffer
                .push_back((self.line_number, content.to_string()));
            if self.context_buffer.len() > self.context_size {
                self.context_buffer.pop_front();
            }
        }

        Ok(())
    }

    fn finish<W: Write>(&mut self, output: &mut Output<W>, reason: FinishReason) -> io::Result<()> {
        let total_lines = self.line_number;
        if total_lines == 0 {
            return output.flush();
        }

        let tail_start = if total_lines > self.last_count {
            total_lines - self.last_count + 1
        } else {
            1
        };
        let needs_truncation = total_lines > self.first_count + self.last_count;

        if let Some(marker) = self.final_marker(reason, tail_start, needs_truncation) {
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

    fn final_marker(
        &self,
        reason: FinishReason,
        tail_start: usize,
        needs_truncation: bool,
    ) -> Option<String> {
        if self.pattern.is_some() {
            return self.final_pattern_marker(reason, tail_start, needs_truncation);
        }

        self.final_default_marker(reason, needs_truncation)
    }

    fn final_pattern_marker(
        &self,
        reason: FinishReason,
        tail_start: usize,
        needs_truncation: bool,
    ) -> Option<String> {
        if self.matches_shown > 0 {
            let gap_start = self.last_output_line + 1;
            let gap_end = tail_start;
            let lines_truncated = gap_end.saturating_sub(gap_start);
            let remaining_matches = self.total_matches - self.matches_shown;

            if lines_truncated == 0 && remaining_matches == 0 {
                return None;
            }

            return Some(match reason {
                FinishReason::Interrupted(_) => {
                    interrupted_marker(lines_truncated, remaining_matches, self.total_matches)
                }
                FinishReason::Completed if remaining_matches > 0 => format!(
                    "[... {} lines and {} matches truncated ({} total) ...]",
                    lines_truncated, remaining_matches, self.total_matches
                ),
                FinishReason::Completed => {
                    format!("[... {} lines truncated ...]", lines_truncated)
                }
            });
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
            if line_number == *end + 1 {
                *end = line_number;
                return;
            }
        }

        self.match_output_ranges.push((line_number, line_number));
    }

    fn was_output_in_match(&self, line_number: usize) -> bool {
        self.match_output_ranges
            .iter()
            .any(|(start, end)| line_number >= *start && line_number <= *end)
    }
}

fn truncate_line(line: &str, width: usize) -> String {
    if width == 0 {
        return line.to_string();
    }

    let char_count = line.chars().count();
    let max_len = width * 2;

    if char_count <= max_len {
        return line.to_string();
    }

    let removed = char_count - max_len;
    let marker = format!("[... {} chars ...]", removed);

    let result_len = width + marker.len() + width;
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
