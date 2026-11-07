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
    assert!(report.contains("6 rendered, 0 reused"), "{report}");
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
    assert_eq!(slides.len(), 6);
    assert_eq!(deck_json["planned"], 7.0);
    assert!((deck_json["aspect"].as_f64().unwrap() - 29.7 / 16.7).abs() < 1e-3);
    assert_eq!(slides[0]["title"], "Hello");
    assert_eq!(slides[0]["speaker"], "Ada");
    assert_eq!(
        slides[0]["notes"],
        "<ul><li>Welcome everyone</li><li>Say what the talk is about</li></ul>"
    );
    assert_eq!(slides[5]["before"], 6.0);
    assert_eq!(slides[2]["before"], 1.0);
    assert_eq!(deck_json["player"]["transition"], "fade");
    let steps: Vec<_> = slides.iter().map(|s| s["step"].as_u64().unwrap()).collect();
    assert_eq!(steps, [1, 1, 2, 3, 1, 1]);
    assert_eq!(slides[2]["transition"]["kind"], "slide");
    let videos = slides[4]["videos"].as_array().unwrap();
    assert_eq!(videos.len(), 1);
    assert_eq!(videos[0]["src"], "media/clip.mp4");
    for key in ["l", "t", "w", "h"] {
        let v = videos[0][key].as_f64().unwrap();
        assert!(v > 0.0 && v < 100.0, "{key} = {v}");
    }
    for slide in slides {
        let img = slide["img"].as_str().unwrap();
        assert!(img.ends_with(".svg"), "{img}");
        let svg = std::fs::read_to_string(dist.join(img)).unwrap();
        assert!(svg.starts_with("<svg"), "{img} is not an SVG");
    }
    let images = std::fs::read_dir(dist.join("slides")).unwrap().count();
    assert_eq!(images, 6);

    // A second build reuses every image.
    let report = build(&deck);
    assert!(report.contains("0 rendered, 6 reused"), "{report}");

    // Switching to JPEG renders every page again and removes the SVGs.
    let toml = deck.join("slidekit.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    let text = text.replace("[output]\n", "[output]\nimages = \"jpeg\"\n");
    std::fs::write(&toml, text).unwrap();
    let report = build(&deck);
    assert!(
        report.contains("6 rendered, 0 reused, 6 removed"),
        "{report}"
    );
    for entry in std::fs::read_dir(dist.join("slides")).unwrap() {
        let path = entry.unwrap().path();
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            bytes.starts_with(&[0xFF, 0xD8]),
            "{} is not a JPEG",
            path.display()
        );
    }
}

#[test]
fn contract_versions() {
    let tmp = tempfile::tempdir().unwrap();
    let deck = tmp.path().join("minimal");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    copy_dir(&root.join("examples/minimal"), &deck);
    let lib = deck.join("lib/slidekit.typ");
    let v1 = std::fs::read_to_string(root.join("assets/contracts/v1.typ")).unwrap();
    let current = std::fs::read_to_string(root.join("assets/slidekit.typ")).unwrap();

    // An unmodified old copy is upgraded in place.
    std::fs::write(&lib, &v1).unwrap();
    let report = build(&deck);
    assert!(report.contains("from contract v1 to v2"), "{report}");
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), current);
    assert!(!build(&deck).contains("contract"));

    // A modified old copy is left alone; the deck uses build steps, which v1 lacks, so the
    // compile error comes with a hint to run `slidekit lib`.
    std::fs::write(&lib, format!("{v1}\n// tweaked\n")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_slidekit"))
        .arg("build")
        .arg(&deck)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("does not contain `stepped`")
            && stderr.contains("hint: the deck's slidekit.typ speaks contract v1")
            && stderr.contains("run `slidekit lib`"),
        "{stderr}"
    );

    // A newer library than the binary asks to update slidekit.
    std::fs::write(
        &lib,
        current.replace("#let contract = 2", "#let contract = 99"),
    )
    .unwrap();
    let report = build(&deck);
    assert!(
        report.contains("v99") && report.contains("Update slidekit"),
        "{report}"
    );
}
