use crate::game::Game;

pub fn serialize(game: &Game) -> String {
    let board = game.dead();
    let width = board.width();
    let output_height = board.height() + 2;
    let mut rows = vec![vec!['.'; width]; output_height];
    for y in 0..output_height {
        for x in 0..width {
            if board.occupied(x, y) {
                rows[y][x] = 'o';
            }
        }
    }
    if let Some(piece) = game.active() {
        for cell in piece.cells() {
            if cell.x >= 0 && cell.x < width as i32 && cell.y >= 0 && cell.y < output_height as i32
            {
                rows[cell.y as usize][cell.x as usize] = 'o';
            }
        }
    }

    let mut output = String::new();
    for y in (0..output_height).rev() {
        for x in 0..width {
            output.push(rows[y][x]);
        }
        output.push('\n');
    }
    output
}

pub fn serialize_multiplayer(games: &[Game; 2]) -> String {
    let [left, right] = games.each_ref().map(serialize);
    let mut output = String::new();
    for (a, b) in left.lines().zip(right.lines()) {
        output.push_str(a);
        output.push_str("    ");
        output.push_str(b);
        output.push('\n');
    }
    output
}
