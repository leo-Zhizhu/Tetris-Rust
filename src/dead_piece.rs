use crate::piece::Point;

pub struct DeadPiece {
    width: usize,
    height: usize,
    rows: Vec<Vec<bool>>,
}

impl DeadPiece {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            rows: vec![vec![false; width]; height + 2],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn occupied(&self, x: usize, y: usize) -> bool {
        if y >= self.rows.len() {
            return false;
        }

        self.rows[y][x]
    }

    pub fn can_place(&self, cells: &[Point; 4]) -> bool {
        for cell in cells {
            if cell.x < 0 || cell.x >= self.width as i32 || cell.y < 0 {
                return false;
            }

            if self.occupied(cell.x as usize, cell.y as usize) {
                return false;
            }
        }

        true
    }

    pub fn merge(&mut self, cells: &[Point; 4]) {
        for cell in cells {
            let column = cell.x as usize;
            let row = cell.y as usize;

            while self.rows.len() <= row {
                self.rows.push(vec![false; self.width]);
            }

            self.rows[row][column] = true;
        }
    }

    pub fn clear_full_rows(&mut self) -> usize {
        let row_count = self.rows.len();
        let mut cleared_rows = 0;

        let mut destination_row = 0;
        for source_row in 0..row_count {
            let mut row_is_full = true;
            for &occupied in &self.rows[source_row] {
                if !occupied {
                    row_is_full = false;
                    break;
                }
            }

            if row_is_full {
                cleared_rows += 1;

                continue;
            }

            if destination_row != source_row {
                self.rows[destination_row] = self.rows[source_row].clone();
            }
            destination_row += 1;
        }

        for row in &mut self.rows[destination_row..] {
            row.fill(false);
        }

        cleared_rows
    }
}
