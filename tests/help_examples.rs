use help_test::HelpTest;

const SAMPLE_INPUT: &[u8] = b"line 1\nline 2\nwarning\nline 4\n";

#[test]
fn help_examples() {
    HelpTest::new("trunc")
        .page(&[], |fixture| {
            fixture.stdin(SAMPLE_INPUT);
        })
        .run();
}
