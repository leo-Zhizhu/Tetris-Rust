use crate::game::Game;

pub fn serialize(game: &Game) -> String {
    let board = game.dead();
    let mut rows = vec![vec!['.'; board.width()]; board.height() + 2];
    for (y, row) in rows.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            if board.occupied(x, y) {
                *cell = 'o';
            }
        }
    }
    if let Some(piece) = game.active() {
        for cell in piece.cells() {
            if cell.x >= 0
                && cell.x < board.width() as i32
                && cell.y >= 0
                && cell.y < rows.len() as i32
            {
                rows[cell.y as usize][cell.x as usize] = 'o';
            }
        }
    }

    let mut output = String::new();
    for row in rows.iter().rev() {
        output.extend(row);
        output.push('\n');
    }
    output
}
