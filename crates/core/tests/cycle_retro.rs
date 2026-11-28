use chrono::NaiveDate;
use taskapp_core::{ChangeSource, Db, Error, cycles, history, palette::Look, undo};

type R = Result<(), Box<dyn std::error::Error>>;

fn date(s: &str) -> NaiveDate {
    s.parse().expect("date")
}

#[tokio::test]
async fn retro_fields_save_and_undo() -> R {
    let db = Db::open_in_memory().await?;
    let c = cycles::create(
        &db,
        None,
        date("2026-01-01"),
        date("2026-01-14"),
        Look::default(),
        ChangeSource::Ui,
    )
    .await?;
    assert_eq!(
        (c.goal.clone(), c.goal_met, c.rating, c.notes.as_str()),
        (None, None, None, "")
    );
    let mut set = c.clone();
    set.goal = Some("Ship the importer".into());
    set.goal_met = Some(true);
    set.rating = Some(4);
    set.notes = "Went **well**".into();
    history::step("Retro", async {
        cycles::update(&db, set.clone(), ChangeSource::Ui).await
    })
    .await?;
    assert_eq!(cycles::list(&db).await?[0], set);
    undo::undo(&db).await?;
    assert_eq!(cycles::list(&db).await?[0], c);
    undo::redo(&db).await?;
    assert_eq!(cycles::list(&db).await?[0], set);
    Ok(())
}

#[tokio::test]
async fn rating_outside_one_to_five_is_invalid() -> R {
    let db = Db::open_in_memory().await?;
    let mut c = cycles::create(
        &db,
        None,
        date("2026-01-01"),
        date("2026-01-14"),
        Look::default(),
        ChangeSource::Ui,
    )
    .await?;
    for bad in [0, 6] {
        c.rating = Some(bad);
        let err = cycles::update(&db, c.clone(), ChangeSource::Ui).await;
        assert!(matches!(err, Err(Error::Invalid(_))), "rating {bad}");
    }
    Ok(())
}
