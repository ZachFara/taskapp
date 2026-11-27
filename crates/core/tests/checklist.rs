use taskapp_core::checklist::counts;

#[test]
fn counts_task_lines_done_and_total() {
    assert_eq!(
        counts("intro\n- [ ] one\n- [x] two\n- [X] three\nnot - [ ] a task"),
        (2, 3)
    );
}

#[test]
fn nested_items_count_and_any_bullet_marker() {
    assert_eq!(
        counts("* [ ] parent\n  - [x] child\n    + [ ] grandchild"),
        (1, 3)
    );
}

#[test]
fn fenced_code_is_not_a_checklist() {
    assert_eq!(
        counts("```md\n- [ ] in code\n```\n- [x] real\n~~~\n- [ ] also code\n~~~"),
        (1, 1)
    );
}

#[test]
fn no_list_counts_zero() {
    assert_eq!(counts("just text\n- a bullet"), (0, 0));
}
