use super::*;
use archisim_core::{
    Hand, LevelId, Model, NewOpening, NewSlab, NewWall, Point2, ReferenceLine, WallId, WallSide,
    WallTop,
};
use archisim_library::Library;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "archisim-team-{name}-{}-{}",
        std::process::id(),
        folder::now()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn project() -> (Model, Library, LevelId) {
    let mut m = Model::new("Casa compartida");
    let level = m.add_level("Planta baja", 0.0);
    let lib = archisim_library::add_library(&mut m).unwrap();
    (m, lib, level)
}

fn wall(m: &mut Model, lib: &Library, level: LevelId, a: (f64, f64), b: (f64, f64)) -> WallId {
    m.add_wall(NewWall {
        wall_type: lib.facade,
        level,
        start: Point2::new(a.0, a.1),
        end: Point2::new(b.0, b.1),
        bulge: 0.0,
        reference: ReferenceLine::Center,
        height: 3.0,
        base_offset: 0.0,
        top: WallTop::Unconnected,
    })
    .unwrap()
}

#[test]
fn diff_and_apply_rebuild_the_project() {
    let (mut m, lib, level) = project();
    let w1 = wall(&mut m, &lib, level, (0.0, 0.0), (6.0, 0.0));
    let w2 = wall(&mut m, &lib, level, (6.0, 0.0), (6.0, 4.0));
    let old = doc_of(&m);
    let mut n = m.clone();
    n.move_wall(w1, Point2::new(0.0, -0.5), Point2::new(6.0, -0.5))
        .unwrap();
    n.remove_wall(w2).unwrap();
    let w3 = wall(&mut n, &lib, level, (0.0, 0.0), (0.0, 4.0));
    n.add_opening(NewOpening {
        opening_type: lib.interior_door,
        wall: w3,
        along: 2.0,
        sill: 0.0,
        opens_to: WallSide::Interior,
        hand: Hand::Left,
        setback: None,
    })
    .unwrap();
    let new = doc_of(&n);
    let ops = diff::diff(&old, &new);
    let touched = diff::touched(&ops);
    assert!(touched.contains(&w1.0) && touched.contains(&w2.0) && touched.contains(&w3.0));
    let mut doc = old.clone();
    diff::apply(&mut doc, &ops);
    let rebuilt = model_of(&doc).unwrap();
    assert_eq!(doc_of(&rebuilt), new);
    // Nothing changed, nothing to publish.
    assert!(diff::diff(&new, &new).is_empty());
}

#[test]
fn two_people_work_at_once() {
    let dir = temp_dir("dos");
    let (mut m, lib, level) = project();
    wall(&mut m, &lib, level, (0.0, 0.0), (10.0, 0.0));
    let (mut antonio, mut am) = Session::create(&dir, &m, "Antonio Fernández", "pc").unwrap();
    assert_eq!(antonio.me.slug, "antonio-fernandez");
    let (mut ana, mut bm, _) = Session::open(&dir, "Ana", "mac").unwrap();
    assert_ne!(antonio.me.slot, ana.me.slot);
    assert_ne!(antonio.me.color, ana.me.color);

    // Both add elements at the same time, without seeing each other's.
    let wa = wall(&mut am, &lib, level, (0.0, 0.0), (0.0, 8.0));
    let sb = bm
        .add_slab(NewSlab {
            slab_type: lib.floor_slab,
            level,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(10.0, 0.0),
                Point2::new(10.0, 8.0),
                Point2::new(0.0, 8.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
    assert!(antonio.me.owns_id(wa.0) && ana.me.owns_id(sb.0));
    assert_eq!(antonio.local_ops(&am).len(), 1);

    let (am2, r) = antonio.publish(&am, "Muro oeste", &NoZones).unwrap();
    assert_eq!(r.published, 1);
    let (bm2, r) = ana.publish(&bm, "Forjado", &NoZones).unwrap();
    assert_eq!(r.received.len(), 1, "{r}");
    assert!(r.to_string().contains("Antonio"), "{r}");
    let (am3, r) = antonio.receive(&am2, &NoZones).unwrap();
    assert_eq!(r.received.len(), 1);
    // Both computers end up with the same project.
    assert_eq!(doc_of(&am3), doc_of(&bm2));
    assert_eq!(am3.walls().count(), 2);
    assert_eq!(am3.slabs().count(), 1);
    assert_eq!(antonio.unreceived(), 0);
    assert!(antonio.local_ops(&am3).is_empty());
    // New elements keep taking ids from each one's block.
    let mut am4 = am3.clone();
    let wb = wall(&mut am4, &lib, level, (10.0, 0.0), (10.0, 8.0));
    assert!(antonio.me.owns_id(wb.0) && wb.0 > wa.0);
    // History of an element.
    let h = ana.history(wa.0);
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].0, "Antonio Fernández");
    assert_eq!(ana.log().len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn whoever_reserves_first_keeps_the_element() {
    let dir = temp_dir("reservas");
    let (mut m, lib, level) = project();
    let w = wall(&mut m, &lib, level, (0.0, 0.0), (10.0, 0.0));
    let (mut antonio, am) = Session::create(&dir, &m, "Antonio", "pc").unwrap();
    let (mut ana, bm, _) = Session::open(&dir, "Ana", "mac").unwrap();
    assert!(antonio.reserve(&[w.0], &NoZones).unwrap().is_empty());
    let refused = ana.reserve(&[w.0], &NoZones).unwrap();
    assert_eq!(refused, vec![(w.0, "Antonio".to_string())]);
    assert_eq!(
        ana.owner(w.0, &NoZones).map(|m| m.slug.as_str()),
        Some("antonio")
    );

    // Both move the wall anyway; only the one who has it publishes it.
    let mut am = am;
    am.move_wall(w, Point2::new(0.0, 1.0), Point2::new(10.0, 1.0))
        .unwrap();
    let mut bm = bm;
    bm.move_wall(w, Point2::new(0.0, -1.0), Point2::new(10.0, -1.0))
        .unwrap();
    let (bm, r) = ana.publish(&bm, "Muro sur", &NoZones).unwrap();
    assert_eq!(r.published, 0);
    assert_eq!(r.dropped.len(), 1, "{r}");
    let (_, r) = antonio.publish(&am, "Muro sur", &NoZones).unwrap();
    assert_eq!(r.published, 1);
    let (bm, _) = ana.receive(&bm, &NoZones).unwrap();
    assert!((bm.wall(w).unwrap().start.y - 1.0).abs() < 1e-9);

    // Released, Ana can have it.
    antonio.release(&[w.0]).unwrap();
    assert!(ana.reserve(&[w.0], &NoZones).unwrap().is_empty());
    antonio.refresh().unwrap();
    assert_eq!(antonio.reserved_by().get("ana").map(|s| s.len()), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn changes_that_no_longer_fit_are_left_out_the_same_everywhere() {
    let dir = temp_dir("encaje");
    let (mut m, lib, level) = project();
    let w = wall(&mut m, &lib, level, (0.0, 0.0), (10.0, 0.0));
    let (mut antonio, mut am) = Session::create(&dir, &m, "Antonio", "pc").unwrap();
    let (mut ana, mut bm, _) = Session::open(&dir, "Ana", "mac").unwrap();
    // Ana removes the wall while Antonio puts a door in it.
    bm.remove_wall(w).unwrap();
    am.add_opening(NewOpening {
        opening_type: lib.interior_door,
        wall: w,
        along: 3.0,
        sill: 0.0,
        opens_to: WallSide::Interior,
        hand: Hand::Left,
        setback: None,
    })
    .unwrap();
    let (bm, _) = ana.publish(&bm, "Quitar muro", &NoZones).unwrap();
    let (am, r) = antonio.publish(&am, "Puerta", &NoZones).unwrap();
    assert_eq!(r.published, 0);
    assert_eq!(r.dropped.len(), 1, "{r}");
    assert_eq!(am.openings().count(), 0);
    let (bm, _) = ana.receive(&bm, &NoZones).unwrap();
    assert_eq!(doc_of(&am), doc_of(&bm));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_working_copy_survives_closing() {
    let dir = temp_dir("copia");
    let (mut m, lib, level) = project();
    wall(&mut m, &lib, level, (0.0, 0.0), (10.0, 0.0));
    let (mut antonio, mut am) = Session::create(&dir, &m, "Antonio", "pc").unwrap();
    let w = wall(&mut am, &lib, level, (0.0, 0.0), (0.0, 5.0));
    antonio.save_work(&am).unwrap();
    drop(antonio);
    let (antonio, am, _) = Session::open(&dir, "antonio", "pc").unwrap();
    assert!(am.wall(w).is_some(), "the unpublished wall is still there");
    assert_eq!(antonio.local_ops(&am).len(), 1);
    // And new ids go on after it.
    assert!(am.next_id() > w.0);
    assert_eq!(members(&dir).unwrap().len(), 1);
    assert!(team_folder_of(&dir.join(TEAM_FILE)).is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zones_messages_and_presence() {
    let dir = temp_dir("zonas");
    let (m, _, level) = project();
    let (mut antonio, _) = Session::create(&dir, &m, "Antonio", "pc").unwrap();
    let (mut ana, _, _) = Session::open(&dir, "Ana", "mac").unwrap();
    let sq = |x: f64| vec![[x, 0.0], [x + 5.0, 0.0], [x + 5.0, 5.0], [x, 5.0]];
    antonio
        .reserve_zone("Fachada norte", level.0, sq(0.0))
        .unwrap();
    assert!(ana.reserve_zone("Cocina", level.0, sq(3.0)).is_err());
    ana.reserve_zone("Garaje", level.0, sq(6.0)).unwrap();
    assert!(antonio.refresh().unwrap().reservations);
    assert_eq!(antonio.zones().len(), 2);

    struct Here;
    impl Locator for Here {
        fn in_zone(&self, id: u64, z: &Zone) -> bool {
            id == 7 && z.name == "Garaje"
        }
    }
    assert_eq!(
        antonio.owner(7, &Here).map(|m| m.name.as_str()),
        Some("Ana")
    );
    assert_eq!(antonio.reserve(&[7], &Here).unwrap().len(), 1);

    antonio
        .send("ana", "¿Me dejas el muro del garaje?", &[7])
        .unwrap();
    ana.refresh().unwrap();
    assert_eq!(ana.unread(), 1);
    assert_eq!(ana.inbox()[0].1.ids, vec![7]);
    ana.mark_read().unwrap();
    assert_eq!(ana.unread(), 0);

    antonio.touch(Some(level.0), None, true).unwrap();
    ana.refresh().unwrap();
    assert!(ana.is_online("antonio"));
    assert!(!ana.open_elsewhere());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn slugs() {
    assert_eq!(slug("  José María Núñez "), "jose-maria-nunez");
    assert_eq!(slug("ÁLVARO"), "alvaro");
    assert_eq!(slug("!!"), "persona");
    assert!(zones_overlap(&[], &[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]));
}
