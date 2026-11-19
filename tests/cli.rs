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

#[test]
fn bot_matches_replay_and_good_bot_wins_in_either_slot() {
    use p5_tetris::{
        game::Game, input::parse_actions, multiplayer::MultiplayerGame, resources::Resources,
    };
    use std::path::Path;

    let exe = env!("CARGO_BIN_EXE_p5_tetris");
    let list = Command::new(exe).arg("--list-bots").output().unwrap();
    assert!(list.status.success());
    assert_eq!(list.stdout, b"good-bot\nbad-bot\n");

    let dir = std::env::temp_dir().join(format!("p5-bot-match-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let files = [dir.join("p1.txt"), dir.join("p2.txt")];
    for good in 0..2 {
        let mut players = ["bad-bot"; 2];
        players[good] = "good-bot";
        let result = Command::new(exe)
            .args([
                "--trace-bot-run",
                "--resources",
                "resources",
                "--p1",
                players[0],
                "--p2",
                players[1],
                "--o1",
            ])
            .arg(&files[0])
            .arg("--o2")
            .arg(&files[1])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let traces = files
            .each_ref()
            .map(|p| parse_actions(&fs::read(p).unwrap()));
        assert_eq!(traces[0].len(), traces[1].len());
        let mut game = MultiplayerGame::new(std::array::from_fn(|_| {
            Resources::load(Path::new("resources"))
        }));
        for (&p1, &p2) in traces[0].iter().zip(&traces[1]) {
            assert!(
                !game.games().iter().any(Game::is_over),
                "trace continued after top-out"
            );
            game.step([p1, p2]);
        }
        assert!(!game.games()[good].is_over());
        assert!(game.games()[1 - good].is_over());
        let replay = Command::new(exe)
            .args(["--resources", "resources", "--simulate-inputs-multi-player"])
            .arg(&files[0])
            .arg("--simulate-inputs-multi-player")
            .arg(&files[1])
            .output()
            .unwrap();
        assert!(replay.status.success());
        assert_eq!(result.stdout, replay.stdout);
    }
    for file in files {
        fs::remove_file(file).unwrap();
    }
    fs::remove_dir(dir).unwrap();
}
