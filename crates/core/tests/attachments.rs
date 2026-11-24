use taskapp_core::{Db, attachments};

type R = Result<(), Box<dyn std::error::Error>>;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest-of-image";

#[tokio::test]
async fn stores_once_and_reads_back() -> R {
    let db = Db::open_in_memory().await?;
    let a = attachments::add(&db, PNG, "image/png").await?;
    let b = attachments::add(&db, PNG, "image/png").await?;
    assert_eq!(a.id, b.id);
    let got = attachments::get(&db, a.id).await?;
    assert_eq!(
        (got.mime.as_str(), got.bytes.as_slice()),
        ("image/png", PNG)
    );
    assert!(db.files.join(format!("{}.png", a.sha256)).is_file());
    Ok(())
}

#[tokio::test]
async fn refuses_wrong_spoofed_svg_and_big() -> R {
    let db = Db::open_in_memory().await?;
    let jpeg = b"\xff\xd8\xffdata";
    assert!(
        attachments::add(&db, jpeg, "image/png").await.is_err(),
        "spoofed type"
    );
    assert!(
        attachments::add(&db, b"<svg xmlns='x'/>", "image/svg+xml")
            .await
            .is_err()
    );
    assert!(attachments::add(&db, b"hello", "image/png").await.is_err());
    let mut big = PNG.to_vec();
    big.resize(attachments::MAX_BYTES + 1, 0);
    assert!(attachments::add(&db, &big, "image/png").await.is_err());
    assert!(attachments::add(&db, jpeg, "image/jpeg").await.is_ok());
    let mut webp = b"RIFF\0\0\0\0WEBPVP8 ".to_vec();
    webp.push(0);
    assert!(attachments::add(&db, &webp, "image/webp").await.is_ok());
    Ok(())
}
