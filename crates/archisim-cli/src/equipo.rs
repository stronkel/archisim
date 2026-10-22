//! `archisim equipo`: a shared project folder seen from the command line
//! (who is in it, what they published and reserved), the project as anyone
//! receives it, and a sample folder with two people working on the demo.

use std::path::Path;

use archisim_core::{NewWall, Point2, ReferenceLine, WallTop};
use archisim_team::{NoZones, Session};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn day(t: u64) -> String {
    // Days and time in UTC: enough to tell changes apart.
    let (d, s) = (t / 86_400, t % 86_400);
    // Civil date from days since 1970 (Howard Hinnant's algorithm).
    let z = d as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dd = doy - (153 * mp + 2) / 5 + 1;
    let mm = if mp < 10 { mp + 3 } else { mp - 9 };
    let yy = yoe + era * 400 + i64::from(mm <= 2);
    format!("{dd:02}/{mm:02}/{yy} {:02}:{:02}", s / 3600, s % 3600 / 60)
}

/// Prints what is in a shared folder.
pub fn status(root: &Path) -> Res {
    let (s, model, skipped) = Session::snapshot(root)?;
    println!("Proyecto de equipo «{}» en {}", s.team.name, root.display());
    println!(
        "  Compartido por {} el {}",
        s.team.created_by,
        day(s.team.created)
    );
    let reserved = s.reserved_by();
    println!("  Personas:");
    for m in s.members() {
        let n = reserved.get(&m.slug).map_or(0, |x| x.len());
        let z = s.zones().iter().filter(|(o, _)| o.slug == m.slug).count();
        let seen = s.presence_of(&m.slug).map_or_else(
            || "nunca conectado".into(),
            |p| format!("visto el {}", day(p.time)),
        );
        let pl =
            |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
        println!(
            "    {} ({}): {} y {} reservados · {seen}",
            m.name,
            m.color,
            pl(n, "elemento", "elementos"),
            pl(z, "zona", "zonas")
        );
    }
    let log = s.log();
    println!("  Cambios publicados: {}", log.len());
    for (who, t, note, n) in log.iter().take(20) {
        println!(
            "    {} · {who}: {note} ({n} elemento{})",
            day(*t),
            if *n == 1 { "" } else { "s" }
        );
    }
    for z in skipped {
        println!("  Aviso: {z}");
    }
    println!(
        "  El proyecto recibido tiene {} muros, {} forjados, {} puertas y ventanas y {} objetos.",
        model.walls().count(),
        model.slabs().count(),
        model.openings().count(),
        model.objects().count()
    );
    Ok(())
}

/// Writes the project as anyone receives it now (and its IFC).
pub fn export(root: &Path, out: &Path) -> Res {
    let (_, model, skipped) = Session::snapshot(root)?;
    for z in skipped {
        eprintln!("Aviso: {z}");
    }
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    archisim_file::save(&model, out)?;
    println!("Proyecto recibido: {}", out.display());
    Ok(())
}

/// A shared folder made from the demo house: Antonio shares it and
/// reserves the façade; Ana reserves the garden as a zone and draws in it;
/// both publish; Ana asks for a wall. Writes `<dir>/equipo/` (the folder,
/// to open with the app) and `<dir>/equipo-recibido.archisim` and `.ifc`.
pub fn sample(dir: &Path) -> Res {
    let root = dir.join("equipo");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(dir)?;
    let doc = crate::demo::build()?;
    let base = doc.model().clone();
    let (mut antonio, mut am) = Session::create(&root, &base, "Antonio Fernández", "estudio-pc")?;
    let (mut ana, mut bm, _) = Session::open(&root, "Ana García", "portatil")?;

    let level = base.levels()[0].id;
    let partition = base
        .wall_types()
        .min_by(|a, b| a.thickness().total_cmp(&b.thickness()))
        .map(|t| t.id)
        .ok_or("la demo no tiene tipos de muro")?;
    let wall = |a: (f64, f64), b: (f64, f64)| NewWall {
        wall_type: partition,
        level,
        start: Point2::new(a.0, a.1),
        end: Point2::new(b.0, b.1),
        bulge: 0.0,
        reference: ReferenceLine::Center,
        height: 2.6,
        base_offset: 0.0,
        top: WallTop::Unconnected,
    };

    // Antonio reserves the first walls of the house and draws a wall.
    let ids: Vec<u64> = am.walls_on_level(level).map(|w| w.id.0).take(3).collect();
    antonio.reserve(&ids, &NoZones)?;
    let new = am.add_wall(wall((-6.0, -6.0), (-6.0, -2.0)))?;
    antonio.reserve(&[new.0], &NoZones)?;
    let (am2, r) = antonio.publish(&am, "Tapia del jardín", &NoZones)?;
    println!("Antonio: {r}");

    // Ana reserves the garden as a zone and draws a garden wall in it.
    ana.reserve_zone(
        "Jardín",
        level.0,
        vec![[-20.0, -20.0], [-8.0, -20.0], [-8.0, -10.0], [-20.0, -10.0]],
    )?;
    let w = bm.add_wall(wall((-18.0, -18.0), (-10.0, -18.0)))?;
    ana.reserve(&[w.0], &NoZones)?;
    let (bm, r) = ana.publish(&bm, "Muro del huerto", &NoZones)?;
    println!("Ana: {r}");
    if let Some(first) = ids.first() {
        ana.send(
            &antonio.me.slug,
            "¿Me dejas el muro de la fachada sur?",
            &[*first],
        )?;
    }
    let (am3, r) = antonio.receive(&am2, &NoZones)?;
    println!("Antonio recibe: {r}");
    antonio.touch(Some(level.0), None, false)?;
    ana.touch(Some(level.0), None, false)?;
    if archisim_team::doc_of(&bm) != archisim_team::doc_of(&am3) {
        let (bm, _) = ana.receive(&bm, &NoZones)?;
        if archisim_team::doc_of(&bm) != archisim_team::doc_of(&am3) {
            return Err("Antonio y Ana no ven el mismo proyecto".into());
        }
    }

    let out = dir.join(format!("equipo-recibido.{}", archisim_file::EXTENSION));
    archisim_file::save(&am3, &out)?;
    std::fs::write(
        dir.join("equipo-recibido.ifc"),
        crate::ifc_text(&am3, "equipo-recibido", None),
    )?;
    std::fs::write(
        dir.join("equipo-recibido-ifc4.ifc"),
        crate::ifc_text_in(&am3, "equipo-recibido", None, archisim_ifc::Schema::Ifc4),
    )?;
    status(&root)?;
    println!(
        "  Carpeta: {} (ábrela en la aplicación con Colaboración → Abrir proyecto de equipo)",
        root.display()
    );
    println!("  Recibido: {} y equipo-recibido.ifc", out.display());
    Ok(())
}
