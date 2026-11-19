use std::{fs, path::Path, process::Command};

use p5_tetris::{input::parse_actions, multiplayer::MultiplayerGame, resources::Resources};

#[test]
fn supplied_single_player_input() {
    let cases = [
        "better-seq-bot",
        "drop-after-doing-nothing",
        "drop-two-after-doing-nothing",
        "hard-drop",
        "hard-drop-right-hard-drop",
        "last-minute-soft-drop",
        "line-clear",
        "move-left",
        "move-right",
        "rotate-counter-clockwise",
        "seq-bot",
        "soft-drop",
        "start-soft-drop-but-stop",
        "topped-out",
        "wall-kick",
        "worse-seq-bot",
    ];
    for name in cases {
        let input = format!("cli-tests/single-player/{name}.txt");
        let output = Command::new("./target/release/p5_tetris")
            .args([
                "--simulate-inputs-single-player",
                &input,
                "--resources",
                "resources",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let actual = String::from_utf8(output.stdout).unwrap();
        let expected =
            fs::read_to_string(format!("cli-tests/single-player/{name}.expect")).unwrap();
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn supplied_multiplayer_input() {
    let cases = [
        "better-seq-bot",
        "garbage-now",
        "no-garbage-yet",
        "seq-bot",
        "wall-kick",
        "worse-seq-bot",
    ];
    for name in cases {
        let text = fs::read_to_string(format!("cli-tests/multi-player/{name}.txt")).unwrap();
        let mut lines = Vec::new();
        for line in text.lines() {
            lines.push(line);
        }
        fs::write("target/multi-player-1.txt", lines[0]).unwrap();
        fs::write("target/multi-player-2.txt", lines[lines.len() - 1]).unwrap();
        let output = Command::new("./target/release/p5_tetris")
            .args([
                "--resources",
                "resources",
                "--simulate-inputs-multi-player",
                "target/multi-player-1.txt",
                "--simulate-inputs-multi-player",
                "target/multi-player-2.txt",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let actual = String::from_utf8(output.stdout).unwrap();
        let expected = fs::read_to_string(format!("cli-tests/multi-player/{name}.expect")).unwrap();
        assert_eq!(actual, expected, "{name}");
    }
    fs::remove_file("target/multi-player-1.txt").unwrap();
    fs::remove_file("target/multi-player-2.txt").unwrap();
}

#[test]
fn bot_trace_replays_and_good_bot_wins() {
    let list = Command::new("./target/release/p5_tetris")
        .arg("--list-bots")
        .output()
        .unwrap();
    assert!(list.status.success());
    assert_eq!(list.stdout, b"good-bot\nbad-bot\n");
    let result = Command::new("./target/release/p5_tetris")
        .args([
            "--trace-bot-run",
            "--resources",
            "resources",
            "--p1",
            "bad-bot",
            "--p2",
            "good-bot",
            "--o1",
            "target/bot-player-1.txt",
            "--o2",
            "target/bot-player-2.txt",
        ])
        .output()
        .unwrap();
    assert!(result.status.success());
    let player1 = parse_actions(&fs::read("target/bot-player-1.txt").unwrap()).unwrap();
    let player2 = parse_actions(&fs::read("target/bot-player-2.txt").unwrap()).unwrap();
    assert_eq!(player1.len(), player2.len());

    let mut game = MultiplayerGame::new([
        Resources::load(Path::new("resources")),
        Resources::load(Path::new("resources")),
    ]);
    for frame in 0..player1.len() {
        game.step([player1[frame], player2[frame]]);
    }
    assert!(game.games()[0].is_over());
    assert!(!game.games()[1].is_over());

    let replay = Command::new("./target/release/p5_tetris")
        .args([
            "--resources",
            "resources",
            "--simulate-inputs-multi-player",
            "target/bot-player-1.txt",
            "--simulate-inputs-multi-player",
            "target/bot-player-2.txt",
        ])
        .output()
        .unwrap();
    assert!(replay.status.success());
    assert_eq!(result.stdout, replay.stdout);
    fs::remove_file("target/bot-player-1.txt").unwrap();
    fs::remove_file("target/bot-player-2.txt").unwrap();
}
