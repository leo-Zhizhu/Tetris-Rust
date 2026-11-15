use std::{fs, process::Command};

#[test]
fn supplied_single_player_input() {

    let cases = [
        "drop-after-doing-nothing",
        "drop-two-after-doing-nothing",
        "hard-drop",
        "hard-drop-right-hard-drop",
        "last-minute-soft-drop",
        "line-clear",
        "move-left",
        "move-right",
        "rotate-counter-clockwise",
        "soft-drop",
        "start-soft-drop-but-stop",
        "topped-out",
    ];

    for name in cases {
        let input = format!("cli-tests/single-player/{name}.txt");
        let output = Command::new("./target/debug/p5_tetris")
            .arg("--simulate-inputs-single-player")
            .arg(input)
            .args(["--resources", "resources"])
            .output()
            .unwrap();

        let actual = String::from_utf8(output.stdout).unwrap();
        let expected =
            fs::read_to_string(format!("cli-tests/single-player/{name}.expect")).unwrap();
        assert_eq!(actual, expected);
    }
}
