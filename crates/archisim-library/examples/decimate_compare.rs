//! Writes a workshop sofa as OBJ, whole and lightened two ways (grid
//! clustering and edge collapse), to compare them: `cargo run --release
//! -p archisim-library --example decimate_compare out/`.
use std::fmt::Write as _;

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or("out".into()));
    std::fs::create_dir_all(&out).unwrap();
    let mut p = archisim_props::Params::default();
    p.set("style", 8.0);
    let pm = archisim_props::make("fine_sofa", &p).unwrap();
    let mut mesh = archisim_mesh::Mesh {
        positions: pm.positions.clone(),
        colors: vec![[180, 180, 180]; pm.positions.len()],
        ..Default::default()
    };
    for (role, tris) in &pm.parts {
        mesh.materials.push(archisim_mesh::Material {
            name: role.to_string(),
            color: [180, 180, 180],
            alpha: 1.0,
            texture: None,
        });
        mesh.groups.push(archisim_mesh::Group {
            material: mesh.materials.len() - 1,
            triangles: tris.clone(),
        });
    }
    let write = |name: &str, m: &archisim_mesh::Mesh| {
        let mut s = String::new();
        let mut mtl = String::new();
        writeln!(s, "mtllib {name}.mtl").unwrap();
        for p in &m.positions {
            writeln!(s, "v {} {} {}", p[0], p[1], p[2]).unwrap();
        }
        for g in &m.groups {
            let mat = &m.materials[g.material];
            writeln!(s, "usemtl {}", mat.name).unwrap();
            for t in &g.triangles {
                writeln!(s, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1).unwrap();
            }
        }
        for (i, mat) in m.materials.iter().enumerate() {
            let h = (i as f32 * 0.37) % 1.0;
            writeln!(
                mtl,
                "newmtl {}\nKd {} {} {}",
                mat.name,
                0.4 + 0.5 * h,
                0.35 + 0.3 * (1.0 - h),
                0.3
            )
            .unwrap();
        }
        std::fs::write(out.join(format!("{name}.obj")), s).unwrap();
        std::fs::write(out.join(format!("{name}.mtl")), mtl).unwrap();
        println!("{name}: {}", m.triangle_count());
    };
    write("0-entero", &mesh);
    for t in [4000, 1200] {
        write(&format!("rejilla-{t}"), &archisim_mesh::simplify(&mesh, t));
        write(
            &format!("aristas-{t}"),
            &archisim_mesh::decimate(&mesh, t).unwrap(),
        );
    }
}
