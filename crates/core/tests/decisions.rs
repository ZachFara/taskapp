use taskapp_core::{
    ChangeSource, Db, DecisionInput, DecisionMode, DecisionOption, DecisionStatus, Error, IssueId,
    NewIssue, decisions, history, issues, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

fn opt(key: &str) -> DecisionOption {
    DecisionOption {
        key: key.into(),
        label: format!("Option {key}"),
        tradeoff: "cheap".into(),
    }
}

fn input(q: &str) -> DecisionInput {
    DecisionInput {
        question: q.into(),
        context: "ctx".into(),
        options: vec![opt("a"), opt("b")],
        recommendation: Some("a".into()),
        mode: DecisionMode::Assume,
        source: None,
        external_id: None,
        issue_ids: vec![],
    }
}

async fn issue(db: &Db) -> Result<IssueId, Error> {
    let new = NewIssue {
        title: "ABC one".into(),
        ..NewIssue::default()
    };
    Ok(issues::create(db, new, UI).await?.id)
}

#[tokio::test]
async fn create_validates_and_lists_blocking_first() -> R {
    let db = Db::open_in_memory().await?;
    let first = decisions::create(&db, input("Which?"), ChangeSource::Cli).await?;
    assert_eq!(
        (first.source.as_str(), first.status),
        ("cli", DecisionStatus::Open)
    );
    let mut b = input("Urgent?");
    b.mode = DecisionMode::Blocking;
    let urgent = decisions::create(&db, b, UI).await?;
    assert_eq!(urgent.source, "ui");
    let open = decisions::list(&db, Some(DecisionStatus::Open)).await?;
    assert_eq!(
        open.iter().map(|d| d.id).collect::<Vec<_>>(),
        vec![urgent.id, first.id]
    );

    let mut bad = input(" ");
    assert!(decisions::create(&db, bad.clone(), UI).await.is_err());
    bad = input("q");
    bad.recommendation = Some("zz".into());
    assert!(decisions::create(&db, bad.clone(), UI).await.is_err());
    bad = input("q");
    bad.options.push(opt("a"));
    assert!(decisions::create(&db, bad, UI).await.is_err());
    Ok(())
}

#[tokio::test]
async fn external_id_is_unique_per_source() -> R {
    let db = Db::open_in_memory().await?;
    let mut a = input("One");
    a.source = Some("curia".into());
    a.external_id = Some("D-1".into());
    decisions::create(&db, a.clone(), UI).await?;
    assert!(matches!(
        decisions::create(&db, a.clone(), UI).await,
        Err(Error::Invalid(_))
    ));
    a.source = Some("other".into());
    decisions::create(&db, a, UI).await?;
    Ok(())
}

#[tokio::test]
async fn answer_by_option_or_text_once() -> R {
    let db = Db::open_in_memory().await?;
    let d = decisions::create(&db, input("Q"), UI).await?;
    let (none, both) = (None::<String>, Some("a".to_string()));
    assert!(
        decisions::answer(&db, d.id, none.clone(), None, String::new(), UI)
            .await
            .is_err()
    );
    assert!(
        decisions::answer(&db, d.id, both.clone(), both.clone(), String::new(), UI)
            .await
            .is_err()
    );
    assert!(
        decisions::answer(&db, d.id, Some("nope".into()), None, String::new(), UI)
            .await
            .is_err()
    );
    let done = decisions::answer(&db, d.id, both.clone(), None, "because".into(), UI).await?;
    assert_eq!(
        (done.status, done.answer.as_deref(), done.note.as_str()),
        (DecisionStatus::Answered, Some("a"), "because")
    );
    assert!(done.answered_at.is_some());
    assert!(
        decisions::answer(&db, d.id, both, None, String::new(), UI)
            .await
            .is_err()
    );
    assert!(decisions::withdraw(&db, d.id, UI).await.is_err());

    let e = decisions::create(&db, input("Q2"), UI).await?;
    let free = decisions::answer(
        &db,
        e.id,
        None,
        Some("something else".into()),
        String::new(),
        UI,
    )
    .await?;
    assert_eq!(free.answer.as_deref(), Some("something else"));
    let w = decisions::create(&db, input("Q3"), UI).await?;
    assert_eq!(
        decisions::withdraw(&db, w.id, UI).await?.status,
        DecisionStatus::Withdrawn
    );
    assert_eq!(
        decisions::list(&db, Some(DecisionStatus::Open))
            .await?
            .len(),
        0
    );
    assert_eq!(decisions::list(&db, None).await?.len(), 3);
    Ok(())
}

#[tokio::test]
async fn links_and_undo() -> R {
    let db = Db::open_in_memory().await?;
    let i = issue(&db).await?;
    let mut inp = input("Q");
    inp.issue_ids = vec![i];
    let d = history::step("Create decision", decisions::create(&db, inp, UI)).await?;
    assert_eq!(d.issue_ids, vec![i]);
    history::step("Unlink", decisions::unlink(&db, d.id, i, UI)).await?;
    assert_eq!(decisions::get(&db, d.id).await?.issue_ids, vec![]);
    assert!(decisions::link(&db, d.id, IssueId(999), UI).await.is_err());
    history::step("Link", decisions::link(&db, d.id, i, UI)).await?;
    history::step(
        "Answer",
        decisions::answer(&db, d.id, Some("b".into()), None, String::new(), UI),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(
        decisions::get(&db, d.id).await?.status,
        DecisionStatus::Open
    );
    undo::undo(&db).await?;
    assert_eq!(decisions::get(&db, d.id).await?.issue_ids, vec![]);
    undo::undo(&db).await?;
    assert_eq!(decisions::get(&db, d.id).await?.issue_ids, vec![i]);
    undo::undo(&db).await?;
    assert!(decisions::get(&db, d.id).await.is_err());
    Ok(())
}
