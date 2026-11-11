//! `archisim equipar`: the line of orders of `archisim_equip`.

use std::path::Path;

use archisim_equip::{equip, Res};

pub fn main(args: &[String]) -> Res {
    let mut objects = None;
    let mut collection = None;
    let mut porches = Vec::new();
    let mut n50 = None;
    let mut uw = None;
    let mut plain = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--objetos" => {
                objects = args.get(i + 1).cloned();
                i += 2;
            }
            "--porche" => {
                if let Some(p) = args.get(i + 1) {
                    porches.push(p.clone());
                }
                i += 2;
            }
            "--n50" => {
                n50 = args
                    .get(i + 1)
                    .and_then(|v| v.replace(',', ".").parse::<f64>().ok());
                i += 2;
            }
            "--uw" => {
                uw = args
                    .get(i + 1)
                    .and_then(|v| v.replace(',', ".").parse::<f64>().ok());
                i += 2;
            }
            "--coleccion" => {
                collection = args.get(i + 1).cloned();
                i += 2;
            }
            _ => {
                plain.push(args[i].clone());
                i += 1;
            }
        }
    }
    let (Some(src), Some(out)) = (plain.first(), plain.get(1)) else {
        return Err("Uso: archisim equipar <proyecto> <salida.archisim> [--objetos lista.csv] [--coleccion «Nórdico cálido»] [--porche MOD-07:sur:3,0:7,2] [--n50 3] [--uw 1,4]".into());
    };
    let mut m = archisim_file::open(Path::new(src))?;
    let list = objects.map(std::fs::read_to_string).transpose()?;
    let t = equip(
        &mut m,
        list.as_deref(),
        collection.as_deref(),
        &porches,
        n50,
        uw,
    )?;
    print!("{t}");
    archisim_file::save(&m, Path::new(out))?;
    println!("Guardado: {out}");
    Ok(())
}
