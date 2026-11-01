//! Terrain grids in ESRI ASCII (`.asc`), as the CNIG gives its digital
//! terrain models (MDT02, MDT05) once converted, or many GIS export them:
//! a header (`ncols`, `nrows`, `xllcorner` or `xllcenter`, `yllcorner` or
//! `yllcenter`, `cellsize`, `NODATA_value`) and the heights row by row
//! from the north.

/// A grid of heights.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    /// The centre of the south-west cell, map metres.
    pub x0: f64,
    pub y0: f64,
    pub cell: f64,
    /// Row by row from the south (row 0 the southern one); `None`: no data.
    pub z: Vec<Option<f64>>,
}

impl Grid {
    /// The height at a cell (column from the west, row from the south).
    pub fn at(&self, col: usize, row: usize) -> Option<f64> {
        (col < self.cols && row < self.rows)
            .then(|| self.z[row * self.cols + col])
            .flatten()
    }

    /// The map point of a cell's centre.
    pub fn point(&self, col: usize, row: usize) -> [f64; 2] {
        [
            self.x0 + col as f64 * self.cell,
            self.y0 + row as f64 * self.cell,
        ]
    }
}

/// Reads an ESRI ASCII grid.
pub fn read(text: &str) -> Result<Grid, String> {
    let mut words = text.split_whitespace().peekable();
    let mut cols = 0usize;
    let mut rows = 0usize;
    let (mut x, mut y, mut cell) = (0.0, 0.0, 0.0);
    let mut corner = (true, true);
    let mut nodata: Option<f64> = None;
    // The header: keywords and values, until a number comes first.
    while let Some(w) = words.peek() {
        if w.parse::<f64>().is_ok() {
            break;
        }
        let key = words.next().unwrap_or_default().to_ascii_lowercase();
        let value: f64 = words
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or(format!("cabecera sin valor para «{key}»"))?;
        match key.as_str() {
            "ncols" => cols = value as usize,
            "nrows" => rows = value as usize,
            "xllcorner" => (x, corner.0) = (value, true),
            "xllcenter" => (x, corner.0) = (value, false),
            "yllcorner" => (y, corner.1) = (value, true),
            "yllcenter" => (y, corner.1) = (value, false),
            "cellsize" => cell = value,
            "nodata_value" => nodata = Some(value),
            _ => {}
        }
    }
    if cols == 0 || rows == 0 || cell <= 0.0 {
        return Err("cabecera incompleta (ncols, nrows, cellsize)".into());
    }
    let values: Vec<f64> = words.filter_map(|w| w.parse().ok()).collect();
    if values.len() < cols * rows {
        return Err(format!(
            "faltan alturas: {} de {}",
            values.len(),
            cols * rows
        ));
    }
    // From the north in the file; from the south here.
    let mut z = vec![None; cols * rows];
    for r in 0..rows {
        for c in 0..cols {
            let v = values[r * cols + c];
            let keep = nodata.is_none_or(|n| (v - n).abs() > 1e-9);
            z[(rows - 1 - r) * cols + c] = keep.then_some(v);
        }
    }
    Ok(Grid {
        cols,
        rows,
        x0: x + if corner.0 { cell / 2.0 } else { 0.0 },
        y0: y + if corner.1 { cell / 2.0 } else { 0.0 },
        cell,
        z,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_grid() {
        let g = read(
            "ncols 3\nnrows 2\nxllcorner 100\nyllcorner 200\ncellsize 5\nNODATA_value -9999\n1 2 3\n4 -9999 6\n",
        )
        .unwrap();
        assert_eq!((g.cols, g.rows), (3, 2));
        assert_eq!(g.point(0, 0), [102.5, 202.5]);
        // The last line of the file is the southern row.
        assert_eq!(g.at(0, 0), Some(4.0));
        assert_eq!(g.at(1, 0), None);
        assert_eq!(g.at(2, 1), Some(3.0));
    }
}
