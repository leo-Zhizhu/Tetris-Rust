use crate::game::Game;

pub fn serialize(game: &Game) -> String {
    let board = game.dead();
    let width = board.width();
    let output_height = board.height() + 2;
    let mut rows = vec![vec!['.'; width]; output_height];

    for row in 0..output_height {
        for column in 0..width {
            if board.occupied(column, row) {
                rows[row][column] = 'o';
            }
        }
    }
    if let Some(piece) = game.active() {
        for cell in piece.cells() {
            if cell.x < 0 || cell.x >= width as i32 {
                continue;
            }
            if cell.y < 0 || cell.y >= output_height as i32 {
                continue;
            }
            rows[cell.y as usize][cell.x as usize] = 'o';
        }
    }

    let mut output = String::new();
    let mut row = output_height;
    while row > 0 {
        row -= 1;
        for column in 0..width {
            output.push(rows[row][column]);
        }
        output.push('\n');
    }
    output
}

pub fn serialize_multiplayer(games: &[Game; 2]) -> String {
    let left = serialize(&games[0]);
    let right = serialize(&games[1]);
    let left_rows: Vec<&str> = left.lines().collect();
    let right_rows: Vec<&str> = right.lines().collect();
    let mut output = String::new();
    let mut row = 0;

    while row < left_rows.len() && row < right_rows.len() {
        output.push_str(left_rows[row]);
        output.push_str("    ");
        output.push_str(right_rows[row]);
        output.push('\n');
        row += 1;
    }
    output
}
