//! Builds `examples/minimal` in a temp dir with the real binary and checks the outputs.

use std::path::Path;
use std::process::Command;

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let (from, to) = (entry.path(), dst.join(entry.file_name()));
        if entry.file_name() == "dist" {
            continue;
        }
        if from.is_dir() {
            copy_dir(&from, &to);
        } else {
            std::fs::copy(&from, &to).unwrap();
        }
    }
}

fn build(deck: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_slidekit"))
        .arg("build")
        .arg(deck)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "build failed:\n{stderr}");
    stderr
}

#[test]
fn builds_minimal_example() {
    let tmp = tempfile::tempdir().unwrap();
    let deck = tmp.path().join("minimal");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/minimal"),
        &deck,
    );

    let report = build(&deck);
    assert!(report.contains("4 rendered, 0 reused"), "{report}");
    let dist = deck.join("dist");
    for pdf in ["slides.pdf", "handout.pdf"] {
        let bytes = std::fs::read(dist.join(pdf)).unwrap();
        assert!(bytes.starts_with(b"%PDF-"), "{pdf} is not a PDF");
    }
    assert!(dist.join("media/clip.mp4").is_file());

    let html = std::fs::read_to_string(dist.join("index.html")).unwrap();
    assert!(html.contains("<title>slidekit minimal example</title>"));
    let json = html
        .split("const DECK = ")
        .nth(1)
        .unwrap()
        .split(";\n")
        .next()
        .unwrap();
    let deck_json: serde_json::Value = serde_json::from_str(json).unwrap();
    let slides = deck_json["slides"].as_array().unwrap();
    assert_eq!(slides.len(), 4);
    assert_eq!(deck_json["planned"], 7.0);
    assert!((deck_json["aspect"].as_f64().unwrap() - 29.7 / 16.7).abs() < 1e-3);
    assert_eq!(slides[0]["title"], "Hello");
    assert_eq!(slides[0]["speaker"], "Ada");
    assert_eq!(
        slides[0]["notes"],
        "<ul><li>Welcome everyone</li><li>Say what the talk is about</li></ul>"
    );
    assert_eq!(slides[3]["before"], 6.0);
    let videos = slides[2]["videos"].as_array().unwrap();
    assert_eq!(videos.len(), 1);
    assert_eq!(videos[0]["src"], "media/clip.mp4");
    for key in ["l", "t", "w", "h"] {
        let v = videos[0][key].as_f64().unwrap();
        assert!(v > 0.0 && v < 100.0, "{key} = {v}");
    }
    for slide in slides {
        assert!(dist.join(slide["img"].as_str().unwrap()).is_file());
    }
    let images = std::fs::read_dir(dist.join("slides")).unwrap().count();
    assert_eq!(images, 4);

    // A second build reuses every image.
    let report = build(&deck);
    assert!(report.contains("0 rendered, 4 reused"), "{report}");
}
