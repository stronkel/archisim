//! The plants ArchiSim grows (family «Vegetación», `archisim-flora`):
//! the species of the library, Mediterranean first (those AD5 plants most:
//! stone pine, olive, wild olive, holm oak, cork oak, eucalyptus,
//! chestnut, jacaranda, cypress, orange, hackberry, palms; rosemary,
//! mastic, broom, saltbush, samphire) and others for any climate. Each is
//! a type of the family with its values and its colours as materials; they
//! weigh nothing (a few numbers each), so they all come with the program.

use archisim_core::{
    CutPattern, Model, ModelError, ObjectTypeId, ParamScope, Value, Values, VEGETATION_FAMILY,
};

/// A colour of a species: the material's name and its colour.
type Paint = (&'static str, [u8; 3]);

/// A species of the library.
pub struct FloraEntry {
    pub name: &'static str,
    /// Where it is in the gallery's «Vegetación» folder.
    pub group: &'static str,
    pub values: &'static [(&'static str, V)],
    pub leaves: Paint,
    pub bark: Paint,
    pub flowers: Option<Paint>,
    pub fruit: Option<Paint>,
}

/// A value of a species.
#[derive(Clone, Copy)]
pub enum V {
    N(f64),
    T(&'static str),
}

use V::{N, T};

const BARK: Paint = ("Corteza", [112, 86, 62]);
const BARK_GREY: Paint = ("Corteza gris", [128, 120, 106]);
const BARK_PALE: Paint = ("Corteza clara", [196, 186, 164]);
const STEMS: Paint = ("Tallos", [104, 92, 70]);

macro_rules! sp {
    ($name:literal, $group:literal, $leaves:expr, $bark:expr, $flowers:expr, $fruit:expr, [$($k:literal = $v:expr),* $(,)?]) => {
        FloraEntry {
            name: $name,
            group: $group,
            values: &[$(($k, $v)),*],
            leaves: $leaves,
            bark: $bark,
            flowers: $flowers,
            fruit: $fruit,
        }
    };
}

/// The species of the library, by groups.
pub const FLORA_CATALOGUE: &[FloraEntry] = &[
    // --- Mediterranean trees -----------------------------------------
    sp!(
        "Pino piñonero",
        "Árboles mediterráneos",
        ("Hojas de pino piñonero", [92, 128, 86]),
        ("Corteza de pino", [128, 92, 70]),
        None,
        None,
        [
            "height" = N(14.0),
            "crown" = N(12.0),
            "trunk" = N(0.6),
            "clear" = N(8.0),
            "habit" = T("árbol"),
            "crown_shape" = T("parasol"),
            "foliage" = T("acículas"),
            "lean" = N(0.3),
            "gnarl" = N(0.3),
            "limbs" = N(6.0),
            "levels" = N(3.0),
            "angle" = N(55.0),
            "droop" = N(-0.2),
            "density" = N(0.8),
            "clump" = N(0.8),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Pino carrasco",
        "Árboles mediterráneos",
        ("Hojas de pino carrasco", [110, 140, 92]),
        ("Corteza de pino", [128, 92, 70]),
        None,
        None,
        [
            "height" = N(12.0),
            "crown" = N(8.0),
            "trunk" = N(0.45),
            "clear" = N(4.0),
            "habit" = T("árbol"),
            "crown_shape" = T("irregular"),
            "foliage" = T("acículas"),
            "lean" = N(0.45),
            "gnarl" = N(0.5),
            "limbs" = N(5.0),
            "angle" = N(45.0),
            "density" = N(0.55),
            "clump" = N(0.7),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Olivo",
        "Árboles mediterráneos",
        ("Hojas de olivo", [150, 164, 122]),
        BARK_GREY,
        None,
        Some(("Aceituna", [70, 70, 48])),
        [
            "height" = N(5.0),
            "crown" = N(5.5),
            "trunk" = N(0.45),
            "clear" = N(1.2),
            "habit" = T("árbol"),
            "crown_shape" = T("irregular"),
            "foliage" = T("hoja menuda"),
            "lean" = N(0.3),
            "gnarl" = N(0.85),
            "limbs" = N(4.0),
            "angle" = N(50.0),
            "droop" = N(0.1),
            "density" = N(0.6),
            "clump" = N(0.45),
            "fruit" = N(0.08),
            "style" = T("follaje"),
        ]
    ),
    sp!(
        "Acebuche",
        "Árboles mediterráneos",
        ("Hojas de acebuche", [138, 152, 110]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(4.0),
            "crown" = N(4.5),
            "trunk" = N(0.3),
            "clear" = N(0.8),
            "stems" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("irregular"),
            "foliage" = T("hoja menuda"),
            "lean" = N(0.25),
            "gnarl" = N(0.7),
            "limbs" = N(5.0),
            "angle" = N(45.0),
            "density" = N(0.75),
            "clump" = N(0.35),
            "style" = T("hojas"),
        ]
    ),
    sp!(
        "Encina",
        "Árboles mediterráneos",
        ("Hojas de encina", [86, 112, 74]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(10.0),
            "crown" = N(9.0),
            "trunk" = N(0.6),
            "clear" = N(2.5),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja menuda"),
            "lean" = N(0.15),
            "gnarl" = N(0.4),
            "limbs" = N(5.0),
            "angle" = N(50.0),
            "density" = N(0.85),
            "clump" = N(0.6),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Alcornoque",
        "Árboles mediterráneos",
        ("Hojas de alcornoque", [96, 120, 80]),
        ("Corcho", [156, 112, 80]),
        None,
        None,
        [
            "height" = N(10.0),
            "crown" = N(10.0),
            "trunk" = N(0.7),
            "clear" = N(2.5),
            "habit" = T("árbol"),
            "crown_shape" = T("irregular"),
            "foliage" = T("hoja ancha"),
            "lean" = N(0.2),
            "gnarl" = N(0.6),
            "limbs" = N(5.0),
            "angle" = N(55.0),
            "density" = N(0.7),
            "clump" = N(0.65),
            "style" = T("follaje"),
        ]
    ),
    sp!(
        "Algarrobo",
        "Árboles mediterráneos",
        ("Hojas de algarrobo", [74, 106, 70]),
        BARK,
        None,
        Some(("Algarroba", [110, 70, 40])),
        [
            "height" = N(7.0),
            "crown" = N(8.0),
            "trunk" = N(0.5),
            "clear" = N(1.5),
            "habit" = T("árbol"),
            "crown_shape" = T("extendida"),
            "foliage" = T("hoja ancha"),
            "gnarl" = N(0.5),
            "limbs" = N(5.0),
            "angle" = N(55.0),
            "density" = N(0.85),
            "clump" = N(0.6),
            "fruit" = N(0.12),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Eucalipto",
        "Árboles mediterráneos",
        ("Hojas de eucalipto", [130, 154, 128]),
        BARK_PALE,
        None,
        None,
        [
            "height" = N(22.0),
            "crown" = N(10.0),
            "trunk" = N(0.7),
            "clear" = N(6.0),
            "habit" = T("árbol"),
            "crown_shape" = T("ovalada"),
            "foliage" = T("hoja menuda"),
            "lean" = N(0.2),
            "gnarl" = N(0.3),
            "limbs" = N(5.0),
            "angle" = N(30.0),
            "droop" = N(0.3),
            "density" = N(0.5),
            "clump" = N(0.7),
            "style" = T("dentado"),
        ]
    ),
    sp!(
        "Castaño",
        "Árboles mediterráneos",
        ("Hojas de castaño", [84, 128, 74]),
        BARK,
        None,
        Some(("Erizo de castaña", [150, 160, 70])),
        [
            "height" = N(15.0),
            "crown" = N(12.0),
            "trunk" = N(0.8),
            "clear" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja ancha"),
            "gnarl" = N(0.35),
            "limbs" = N(6.0),
            "angle" = N(45.0),
            "density" = N(0.85),
            "clump" = N(0.8),
            "fruit" = N(0.08),
            "style" = T("ramas y hojas"),
        ]
    ),
    sp!(
        "Almez",
        "Árboles mediterráneos",
        ("Hojas de almez", [104, 146, 84]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(14.0),
            "crown" = N(12.0),
            "trunk" = N(0.6),
            "clear" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("en vaso"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(5.0),
            "angle" = N(35.0),
            "density" = N(0.75),
            "clump" = N(0.6),
            "style" = T("ramas y hojas"),
        ]
    ),
    sp!(
        "Naranjo",
        "Árboles mediterráneos",
        ("Hojas de naranjo", [72, 120, 70]),
        BARK_GREY,
        Some(("Azahar", [248, 246, 236])),
        Some(("Naranja", [236, 140, 30])),
        [
            "height" = N(4.0),
            "crown" = N(4.0),
            "trunk" = N(0.2),
            "clear" = N(0.9),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(4.0),
            "angle" = N(45.0),
            "density" = N(0.95),
            "clump" = N(0.4),
            "fruit" = N(0.5),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Limonero",
        "Árboles mediterráneos",
        ("Hojas de limonero", [96, 138, 74]),
        BARK_GREY,
        Some(("Azahar", [248, 246, 236])),
        Some(("Limón", [238, 214, 70])),
        [
            "height" = N(4.0),
            "crown" = N(4.0),
            "trunk" = N(0.2),
            "clear" = N(0.8),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(4.0),
            "angle" = N(48.0),
            "density" = N(0.9),
            "clump" = N(0.4),
            "fruit" = N(0.45),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Almendro en flor",
        "Árboles mediterráneos",
        ("Hojas de almendro", [124, 158, 96]),
        BARK,
        Some(("Flor de almendro", [244, 214, 224])),
        None,
        [
            "height" = N(6.0),
            "crown" = N(6.0),
            "trunk" = N(0.3),
            "clear" = N(1.2),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja menuda"),
            "gnarl" = N(0.5),
            "limbs" = N(5.0),
            "angle" = N(45.0),
            "density" = N(0.6),
            "clump" = N(0.45),
            "bloom" = N(0.85),
            "style" = T("ramas y hojas"),
        ]
    ),
    sp!(
        "Granado",
        "Árboles mediterráneos",
        ("Hojas de granado", [96, 140, 70]),
        BARK_GREY,
        Some(("Flor de granado", [220, 70, 50])),
        Some(("Granada", [180, 40, 50])),
        [
            "height" = N(4.0),
            "crown" = N(3.5),
            "trunk" = N(0.15),
            "clear" = N(0.5),
            "stems" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja menuda"),
            "gnarl" = N(0.4),
            "limbs" = N(5.0),
            "angle" = N(40.0),
            "density" = N(0.75),
            "clump" = N(0.35),
            "bloom" = N(0.2),
            "fruit" = N(0.25),
            "style" = T("hojas"),
        ]
    ),
    sp!(
        "Higuera",
        "Árboles mediterráneos",
        ("Hojas de higuera", [96, 140, 80]),
        BARK_PALE,
        None,
        None,
        [
            "height" = N(6.0),
            "crown" = N(8.0),
            "trunk" = N(0.4),
            "clear" = N(0.8),
            "stems" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("extendida"),
            "foliage" = T("hoja ancha"),
            "gnarl" = N(0.45),
            "limbs" = N(6.0),
            "angle" = N(60.0),
            "droop" = N(0.2),
            "density" = N(0.8),
            "clump" = N(0.7),
            "style" = T("nube"),
        ]
    ),
    // --- Conifers ----------------------------------------------------
    sp!(
        "Ciprés",
        "Coníferas",
        ("Hojas de ciprés", [52, 86, 64]),
        BARK,
        None,
        None,
        [
            "height" = N(12.0),
            "crown" = N(2.2),
            "trunk" = N(0.35),
            "clear" = N(0.3),
            "habit" = T("conífera"),
            "crown_shape" = T("columnar"),
            "foliage" = T("escamas"),
            "limbs" = N(5.0),
            "levels" = N(2.0),
            "angle" = N(20.0),
            "droop" = N(-0.3),
            "density" = N(0.95),
            "clump" = N(0.5),
            "style" = T("dentado"),
        ]
    ),
    sp!(
        "Ciprés de Arizona",
        "Coníferas",
        ("Hojas de ciprés de Arizona", [110, 140, 130]),
        BARK,
        None,
        None,
        [
            "height" = N(12.0),
            "crown" = N(5.0),
            "trunk" = N(0.4),
            "clear" = N(0.3),
            "habit" = T("conífera"),
            "crown_shape" = T("cónica"),
            "foliage" = T("escamas"),
            "limbs" = N(5.0),
            "levels" = N(2.0),
            "angle" = N(60.0),
            "density" = N(0.9),
            "clump" = N(0.55),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Abeto",
        "Coníferas",
        ("Hojas de abeto", [60, 100, 78]),
        BARK,
        None,
        None,
        [
            "height" = N(20.0),
            "crown" = N(8.0),
            "trunk" = N(0.5),
            "clear" = N(0.5),
            "habit" = T("conífera"),
            "crown_shape" = T("cónica"),
            "foliage" = T("acículas"),
            "limbs" = N(6.0),
            "levels" = N(2.0),
            "angle" = N(80.0),
            "droop" = N(0.2),
            "density" = N(0.9),
            "clump" = N(0.7),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Cedro del Atlas",
        "Coníferas",
        ("Hojas de cedro", [100, 130, 120]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(20.0),
            "crown" = N(12.0),
            "trunk" = N(0.8),
            "clear" = N(2.0),
            "habit" = T("conífera"),
            "crown_shape" = T("cónica"),
            "foliage" = T("acículas"),
            "limbs" = N(5.0),
            "levels" = N(2.0),
            "angle" = N(85.0),
            "droop" = N(-0.1),
            "density" = N(0.7),
            "clump" = N(0.8),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Araucaria",
        "Coníferas",
        ("Hojas de araucaria", [60, 100, 70]),
        BARK,
        None,
        None,
        [
            "height" = N(18.0),
            "crown" = N(7.0),
            "trunk" = N(0.5),
            "clear" = N(3.0),
            "habit" = T("conífera"),
            "crown_shape" = T("cónica"),
            "foliage" = T("acículas"),
            "limbs" = N(6.0),
            "levels" = N(2.0),
            "angle" = N(88.0),
            "droop" = N(-0.2),
            "density" = N(0.5),
            "clump" = N(0.6),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Sabina",
        "Coníferas",
        ("Hojas de sabina", [84, 110, 84]),
        BARK,
        None,
        None,
        [
            "height" = N(4.0),
            "crown" = N(4.0),
            "trunk" = N(0.35),
            "clear" = N(0.3),
            "stems" = N(2.0),
            "habit" = T("conífera"),
            "crown_shape" = T("irregular"),
            "foliage" = T("escamas"),
            "lean" = N(0.3),
            "gnarl" = N(0.8),
            "limbs" = N(4.0),
            "levels" = N(2.0),
            "angle" = N(60.0),
            "density" = N(0.8),
            "clump" = N(0.45),
            "style" = T("follaje"),
        ]
    ),
    // --- Palms ------------------------------------------------------
    sp!(
        "Palmera datilera",
        "Palmeras",
        ("Hojas de palmera datilera", [128, 158, 96]),
        ("Tronco de palmera", [140, 116, 90]),
        None,
        Some(("Dátil", [190, 120, 40])),
        [
            "height" = N(12.0),
            "crown" = N(7.0),
            "trunk" = N(0.5),
            "clear" = N(10.0),
            "habit" = T("palmera"),
            "foliage" = T("pinnada"),
            "lean" = N(0.25),
            "fronds" = N(26.0),
            "frond_length" = N(4.0),
            "fruit" = N(0.4),
            "style" = T("frondas"),
        ]
    ),
    sp!(
        "Palmera canaria",
        "Palmeras",
        ("Hojas de palmera canaria", [104, 146, 84]),
        ("Tronco de palmera", [140, 116, 90]),
        None,
        None,
        [
            "height" = N(12.0),
            "crown" = N(9.0),
            "trunk" = N(0.9),
            "clear" = N(9.0),
            "habit" = T("palmera"),
            "foliage" = T("pinnada"),
            "lean" = N(0.05),
            "droop" = N(0.2),
            "fronds" = N(44.0),
            "frond_length" = N(5.0),
            "style" = T("frondas"),
        ]
    ),
    sp!(
        "Washingtonia",
        "Palmeras",
        ("Hojas de washingtonia", [120, 150, 90]),
        ("Tronco de palmera", [140, 116, 90]),
        None,
        None,
        [
            "height" = N(18.0),
            "crown" = N(4.0),
            "trunk" = N(0.5),
            "clear" = N(16.0),
            "habit" = T("palmera"),
            "foliage" = T("abanico"),
            "lean" = N(0.1),
            "fronds" = N(22.0),
            "frond_length" = N(2.2),
            "style" = T("frondas"),
        ]
    ),
    sp!(
        "Palmito",
        "Palmeras",
        ("Hojas de palmito", [130, 160, 100]),
        ("Tronco de palmera", [140, 116, 90]),
        None,
        None,
        [
            "height" = N(2.5),
            "crown" = N(3.0),
            "trunk" = N(0.25),
            "clear" = N(1.5),
            "habit" = T("palmera"),
            "foliage" = T("abanico"),
            "stems" = N(5.0),
            "fronds" = N(12.0),
            "frond_length" = N(1.3),
            "style" = T("frondas"),
        ]
    ),
    // --- Other trees -----------------------------------------------
    sp!(
        "Jacaranda",
        "Árboles ornamentales",
        ("Hojas de jacaranda", [110, 150, 90]),
        BARK_GREY,
        Some(("Flor de jacaranda", [150, 130, 205])),
        None,
        [
            "height" = N(9.0),
            "crown" = N(9.0),
            "trunk" = N(0.4),
            "clear" = N(2.5),
            "habit" = T("árbol"),
            "crown_shape" = T("extendida"),
            "foliage" = T("hoja menuda"),
            "limbs" = N(5.0),
            "angle" = N(50.0),
            "density" = N(0.55),
            "clump" = N(0.55),
            "bloom" = N(0.75),
            "style" = T("follaje"),
        ]
    ),
    sp!(
        "Castaño de Indias",
        "Árboles ornamentales",
        ("Hojas de castaño de Indias", [84, 128, 74]),
        BARK,
        Some(("Flor de castaño de Indias", [244, 236, 228])),
        None,
        [
            "height" = N(15.0),
            "crown" = N(10.0),
            "trunk" = N(0.7),
            "clear" = N(2.5),
            "habit" = T("árbol"),
            "crown_shape" = T("ovalada"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(6.0),
            "angle" = N(40.0),
            "droop" = N(0.2),
            "density" = N(0.9),
            "clump" = N(0.8),
            "bloom" = N(0.3),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Plátano de sombra",
        "Árboles ornamentales",
        ("Hojas de plátano de sombra", [118, 156, 86]),
        ("Corteza de plátano", [178, 168, 136]),
        None,
        None,
        [
            "height" = N(20.0),
            "crown" = N(14.0),
            "trunk" = N(0.8),
            "clear" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("ovalada"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(6.0),
            "angle" = N(40.0),
            "density" = N(0.8),
            "clump" = N(0.9),
            "style" = T("ramas y hojas"),
        ]
    ),
    sp!(
        "Tipuana",
        "Árboles ornamentales",
        ("Hojas de tipuana", [112, 150, 80]),
        BARK,
        Some(("Flor de tipuana", [232, 200, 70])),
        None,
        [
            "height" = N(15.0),
            "crown" = N(14.0),
            "trunk" = N(0.7),
            "clear" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("extendida"),
            "foliage" = T("hoja menuda"),
            "limbs" = N(6.0),
            "angle" = N(55.0),
            "droop" = N(0.3),
            "density" = N(0.6),
            "clump" = N(0.7),
            "bloom" = N(0.4),
            "style" = T("follaje"),
        ]
    ),
    sp!(
        "Árbol del amor",
        "Árboles ornamentales",
        ("Hojas de árbol del amor", [104, 146, 84]),
        BARK,
        Some(("Flor de árbol del amor", [214, 120, 170])),
        None,
        [
            "height" = N(6.0),
            "crown" = N(6.0),
            "trunk" = N(0.3),
            "clear" = N(1.5),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(5.0),
            "angle" = N(45.0),
            "density" = N(0.6),
            "clump" = N(0.5),
            "bloom" = N(0.8),
            "style" = T("ramas y hojas"),
        ]
    ),
    sp!(
        "Mimosa",
        "Árboles ornamentales",
        ("Hojas de mimosa", [120, 150, 110]),
        BARK_GREY,
        Some(("Flor de mimosa", [240, 220, 60])),
        None,
        [
            "height" = N(8.0),
            "crown" = N(7.0),
            "trunk" = N(0.35),
            "clear" = N(1.5),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja menuda"),
            "limbs" = N(5.0),
            "angle" = N(45.0),
            "density" = N(0.75),
            "clump" = N(0.5),
            "bloom" = N(0.8),
            "style" = T("punteado"),
        ]
    ),
    sp!(
        "Morera",
        "Árboles ornamentales",
        ("Hojas de morera", [110, 158, 80]),
        BARK,
        None,
        None,
        [
            "height" = N(8.0),
            "crown" = N(8.0),
            "trunk" = N(0.45),
            "clear" = N(2.0),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(5.0),
            "angle" = N(50.0),
            "density" = N(0.85),
            "clump" = N(0.7),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Ficus",
        "Árboles ornamentales",
        ("Hojas de ficus", [56, 100, 62]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(12.0),
            "crown" = N(16.0),
            "trunk" = N(1.2),
            "clear" = N(2.5),
            "stems" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("extendida"),
            "foliage" = T("hoja ancha"),
            "gnarl" = N(0.6),
            "limbs" = N(7.0),
            "angle" = N(60.0),
            "density" = N(0.95),
            "clump" = N(1.0),
            "style" = T("follaje"),
        ]
    ),
    sp!(
        "Magnolio",
        "Árboles ornamentales",
        ("Hojas de magnolio", [64, 104, 70]),
        BARK_GREY,
        Some(("Flor de magnolio", [245, 242, 230])),
        None,
        [
            "height" = N(12.0),
            "crown" = N(8.0),
            "trunk" = N(0.5),
            "clear" = N(1.5),
            "habit" = T("árbol"),
            "crown_shape" = T("ovalada"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(5.0),
            "angle" = N(40.0),
            "density" = N(0.95),
            "clump" = N(0.7),
            "bloom" = N(0.15),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Sauce llorón",
        "Árboles ornamentales",
        ("Hojas de sauce", [150, 184, 108]),
        BARK,
        None,
        None,
        [
            "height" = N(10.0),
            "crown" = N(10.0),
            "trunk" = N(0.6),
            "clear" = N(2.0),
            "habit" = T("árbol"),
            "crown_shape" = T("llorona"),
            "foliage" = T("hoja menuda"),
            "limbs" = N(6.0),
            "angle" = N(55.0),
            "droop" = N(0.95),
            "density" = N(0.8),
            "clump" = N(0.6),
            "style" = T("dentado"),
        ]
    ),
    sp!(
        "Chopo",
        "Árboles de ribera y de clima templado",
        ("Hojas de chopo", [124, 162, 92]),
        BARK_PALE,
        None,
        None,
        [
            "height" = N(20.0),
            "crown" = N(4.0),
            "trunk" = N(0.5),
            "clear" = N(1.0),
            "habit" = T("árbol"),
            "crown_shape" = T("columnar"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(6.0),
            "angle" = N(15.0),
            "droop" = N(-0.6),
            "density" = N(0.85),
            "clump" = N(0.5),
            "style" = T("dentado"),
        ]
    ),
    sp!(
        "Álamo blanco",
        "Árboles de ribera y de clima templado",
        ("Hojas de álamo blanco", [170, 192, 150]),
        BARK_PALE,
        None,
        None,
        [
            "height" = N(18.0),
            "crown" = N(10.0),
            "trunk" = N(0.7),
            "clear" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("ovalada"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(6.0),
            "angle" = N(35.0),
            "density" = N(0.75),
            "clump" = N(0.7),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Fresno",
        "Árboles de ribera y de clima templado",
        ("Hojas de fresno", [124, 164, 96]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(14.0),
            "crown" = N(10.0),
            "trunk" = N(0.55),
            "clear" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja menuda"),
            "limbs" = N(5.0),
            "angle" = N(40.0),
            "density" = N(0.65),
            "clump" = N(0.65),
            "style" = T("ramas y hojas"),
        ]
    ),
    sp!(
        "Olmo",
        "Árboles de ribera y de clima templado",
        ("Hojas de olmo", [98, 140, 82]),
        BARK,
        None,
        None,
        [
            "height" = N(16.0),
            "crown" = N(12.0),
            "trunk" = N(0.7),
            "clear" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("en vaso"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(6.0),
            "angle" = N(35.0),
            "density" = N(0.8),
            "clump" = N(0.7),
            "style" = T("ramas y hojas"),
        ]
    ),
    sp!(
        "Tilo",
        "Árboles de ribera y de clima templado",
        ("Hojas de tilo", [128, 168, 96]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(18.0),
            "crown" = N(11.0),
            "trunk" = N(0.7),
            "clear" = N(2.5),
            "habit" = T("árbol"),
            "crown_shape" = T("ovalada"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(6.0),
            "angle" = N(40.0),
            "density" = N(0.9),
            "clump" = N(0.8),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Abedul",
        "Árboles de ribera y de clima templado",
        ("Hojas de abedul", [160, 190, 110]),
        ("Corteza de abedul", [230, 228, 220]),
        None,
        None,
        [
            "height" = N(15.0),
            "crown" = N(7.0),
            "trunk" = N(0.35),
            "clear" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("ovalada"),
            "foliage" = T("hoja menuda"),
            "lean" = N(0.15),
            "limbs" = N(6.0),
            "angle" = N(30.0),
            "droop" = N(0.4),
            "density" = N(0.55),
            "clump" = N(0.5),
            "style" = T("hojas"),
        ]
    ),
    sp!(
        "Arce rojo",
        "Árboles de ribera y de clima templado",
        ("Hojas de arce rojo", [196, 84, 64]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(12.0),
            "crown" = N(9.0),
            "trunk" = N(0.5),
            "clear" = N(2.0),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(5.0),
            "angle" = N(40.0),
            "density" = N(0.8),
            "clump" = N(0.65),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Ginkgo",
        "Árboles de ribera y de clima templado",
        ("Hojas de ginkgo", [226, 200, 92]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(15.0),
            "crown" = N(8.0),
            "trunk" = N(0.5),
            "clear" = N(2.5),
            "habit" = T("árbol"),
            "crown_shape" = T("ovalada"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(5.0),
            "angle" = N(35.0),
            "density" = N(0.6),
            "clump" = N(0.6),
            "style" = T("ramas y hojas"),
        ]
    ),
    // --- Warm and tropical climates ------------------------------------
    sp!(
        "Acacia de sabana",
        "Árboles de clima cálido",
        ("Hojas de acacia", [118, 146, 84]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(8.0),
            "crown" = N(12.0),
            "trunk" = N(0.4),
            "clear" = N(3.5),
            "habit" = T("árbol"),
            "crown_shape" = T("parasol"),
            "foliage" = T("hoja menuda"),
            "lean" = N(0.2),
            "gnarl" = N(0.4),
            "limbs" = N(5.0),
            "angle" = N(65.0),
            "density" = N(0.55),
            "clump" = N(0.6),
            "style" = T("hojas"),
        ]
    ),
    sp!(
        "Árbol tropical",
        "Árboles de clima cálido",
        ("Hojas tropicales", [48, 104, 60]),
        BARK_GREY,
        None,
        None,
        [
            "height" = N(14.0),
            "crown" = N(12.0),
            "trunk" = N(0.8),
            "clear" = N(3.0),
            "habit" = T("árbol"),
            "crown_shape" = T("redonda"),
            "foliage" = T("hoja ancha"),
            "limbs" = N(6.0),
            "angle" = N(50.0),
            "density" = N(1.0),
            "clump" = N(1.0),
            "style" = T("follaje"),
        ]
    ),
    sp!(
        "Drago",
        "Árboles de clima cálido",
        ("Hojas de drago", [90, 128, 96]),
        BARK_PALE,
        None,
        None,
        [
            "height" = N(5.0),
            "crown" = N(4.0),
            "trunk" = N(0.6),
            "clear" = N(3.0),
            "habit" = T("roseta"),
            "foliage" = T("hojas largas"),
            "fronds" = N(40.0),
            "frond_length" = N(0.9),
            "style" = T("frondas"),
        ]
    ),
    // --- Shrubs and low plants -----------------------------------------
    sp!(
        "Romero",
        "Arbustos",
        ("Hojas de romero", [110, 130, 100]),
        STEMS,
        Some(("Flor de romero", [150, 160, 220])),
        None,
        [
            "height" = N(1.0),
            "crown" = N(1.2),
            "trunk" = N(0.04),
            "clear" = N(0.0),
            "stems" = N(9.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("cúpula"),
            "foliage" = T("hoja menuda"),
            "limbs" = N(9.0),
            "levels" = N(2.0),
            "angle" = N(35.0),
            "droop" = N(-0.3),
            "density" = N(0.85),
            "clump" = N(0.14),
            "bloom" = N(0.35),
            "style" = T("hojas"),
        ]
    ),
    sp!(
        "Lentisco",
        "Arbustos",
        ("Hojas de lentisco", [70, 100, 64]),
        STEMS,
        None,
        Some(("Fruto de lentisco", [180, 40, 40])),
        [
            "height" = N(2.0),
            "crown" = N(2.5),
            "trunk" = N(0.08),
            "clear" = N(0.0),
            "stems" = N(8.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("cúpula"),
            "foliage" = T("hoja menuda"),
            "limbs" = N(8.0),
            "levels" = N(2.0),
            "angle" = N(45.0),
            "density" = N(0.95),
            "clump" = N(0.3),
            "fruit" = N(0.1),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Retama",
        "Arbustos",
        ("Tallos de retama", [120, 150, 80]),
        STEMS,
        Some(("Flor de retama", [244, 214, 40])),
        None,
        [
            "height" = N(2.0),
            "crown" = N(2.2),
            "trunk" = N(0.08),
            "clear" = N(0.0),
            "stems" = N(8.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("cúpula"),
            "foliage" = T("retama"),
            "levels" = N(3.0),
            "angle" = N(30.0),
            "droop" = N(0.2),
            "density" = N(0.6),
            "clump" = N(0.15),
            "bloom" = N(0.7),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Retama blanca",
        "Arbustos",
        ("Tallos de retama blanca", [140, 160, 110]),
        STEMS,
        Some(("Flor de retama blanca", [248, 246, 240])),
        None,
        [
            "height" = N(2.5),
            "crown" = N(2.5),
            "trunk" = N(0.08),
            "clear" = N(0.0),
            "stems" = N(9.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("cúpula"),
            "foliage" = T("retama"),
            "levels" = N(3.0),
            "angle" = N(28.0),
            "droop" = N(0.45),
            "density" = N(0.55),
            "clump" = N(0.15),
            "bloom" = N(0.6),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Adelfa",
        "Arbustos",
        ("Hojas de adelfa", [80, 120, 80]),
        STEMS,
        Some(("Flor de adelfa", [230, 130, 170])),
        None,
        [
            "height" = N(3.0),
            "crown" = N(3.0),
            "trunk" = N(0.08),
            "clear" = N(0.0),
            "stems" = N(10.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("cúpula"),
            "foliage" = T("hoja ancha"),
            "levels" = N(2.0),
            "angle" = N(25.0),
            "density" = N(0.8),
            "clump" = N(0.35),
            "bloom" = N(0.45),
            "style" = T("hojas"),
        ]
    ),
    sp!(
        "Mirto",
        "Arbustos",
        ("Hojas de mirto", [70, 105, 70]),
        STEMS,
        Some(("Flor de mirto", [246, 244, 236])),
        None,
        [
            "height" = N(2.0),
            "crown" = N(2.0),
            "trunk" = N(0.06),
            "clear" = N(0.0),
            "stems" = N(8.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("cúpula"),
            "foliage" = T("hoja menuda"),
            "levels" = N(2.0),
            "angle" = N(30.0),
            "density" = N(0.9),
            "clump" = N(0.22),
            "bloom" = N(0.2),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Coscoja",
        "Arbustos",
        ("Hojas de coscoja", [90, 115, 75]),
        STEMS,
        None,
        None,
        [
            "height" = N(1.5),
            "crown" = N(2.0),
            "trunk" = N(0.06),
            "clear" = N(0.0),
            "stems" = N(10.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("irregular"),
            "foliage" = T("hoja menuda"),
            "levels" = N(2.0),
            "angle" = N(45.0),
            "gnarl" = N(0.5),
            "density" = N(0.9),
            "clump" = N(0.22),
            "style" = T("dentado"),
        ]
    ),
    sp!(
        "Pitosporo",
        "Arbustos",
        ("Hojas de pitosporo", [70, 110, 70]),
        STEMS,
        None,
        None,
        [
            "height" = N(3.0),
            "crown" = N(3.0),
            "trunk" = N(0.08),
            "clear" = N(0.0),
            "stems" = N(8.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("cúpula"),
            "foliage" = T("hoja ancha"),
            "levels" = N(2.0),
            "angle" = N(35.0),
            "density" = N(1.0),
            "clump" = N(0.35),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Boj",
        "Arbustos",
        ("Hojas de boj", [60, 100, 60]),
        STEMS,
        None,
        None,
        [
            "height" = N(1.0),
            "crown" = N(1.0),
            "trunk" = N(0.04),
            "clear" = N(0.0),
            "stems" = N(8.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("cúpula"),
            "foliage" = T("escamas"),
            "levels" = N(2.0),
            "angle" = N(40.0),
            "density" = N(1.0),
            "clump" = N(0.15),
            "style" = T("nube"),
        ]
    ),
    sp!(
        "Lavanda",
        "Arbustos",
        ("Hojas de lavanda", [140, 160, 140]),
        STEMS,
        Some(("Flor de lavanda", [150, 120, 200])),
        None,
        [
            "height" = N(0.6),
            "crown" = N(0.8),
            "trunk" = N(0.03),
            "clear" = N(0.0),
            "stems" = N(10.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("cúpula"),
            "foliage" = T("hoja menuda"),
            "levels" = N(2.0),
            "angle" = N(30.0),
            "droop" = N(-0.4),
            "density" = N(0.85),
            "clump" = N(0.09),
            "bloom" = N(0.7),
            "style" = T("punteado"),
        ]
    ),
    sp!(
        "Taray",
        "Arbustos",
        ("Hojas de taray", [130, 150, 120]),
        STEMS,
        Some(("Flor de taray", [230, 180, 200])),
        None,
        [
            "height" = N(4.0),
            "crown" = N(4.0),
            "trunk" = N(0.15),
            "clear" = N(0.0),
            "stems" = N(6.0),
            "habit" = T("arbusto"),
            "crown_shape" = T("irregular"),
            "foliage" = T("escamas"),
            "levels" = N(3.0),
            "angle" = N(35.0),
            "droop" = N(0.4),
            "density" = N(0.6),
            "clump" = N(0.35),
            "bloom" = N(0.35),
            "style" = T("hojas"),
        ]
    ),
    sp!(
        "Tomillo",
        "Matas y plantas de marisma",
        ("Hojas de tomillo", [120, 135, 105]),
        STEMS,
        Some(("Flor de tomillo", [210, 180, 220])),
        None,
        [
            "height" = N(0.3),
            "crown" = N(0.5),
            "trunk" = N(0.02),
            "clear" = N(0.0),
            "stems" = N(12.0),
            "habit" = T("mata"),
            "crown_shape" = T("cojín"),
            "foliage" = T("hoja menuda"),
            "levels" = N(2.0),
            "angle" = N(55.0),
            "density" = N(0.9),
            "clump" = N(0.06),
            "bloom" = N(0.4),
            "style" = T("punteado"),
        ]
    ),
    sp!(
        "Salicornia",
        "Matas y plantas de marisma",
        ("Tallos de salicornia", [110, 140, 80]),
        STEMS,
        Some(("Salicornia roja", [176, 84, 70])),
        None,
        [
            "height" = N(0.35),
            "crown" = N(0.8),
            "trunk" = N(0.02),
            "clear" = N(0.0),
            "stems" = N(14.0),
            "habit" = T("mata"),
            "crown_shape" = T("cojín"),
            "foliage" = T("suculenta"),
            "levels" = N(2.0),
            "angle" = N(55.0),
            "density" = N(0.9),
            "clump" = N(0.07),
            "bloom" = N(0.2),
            "style" = T("punteado"),
        ]
    ),
    sp!(
        "Almajo",
        "Matas y plantas de marisma",
        ("Tallos de almajo", [124, 140, 100]),
        STEMS,
        None,
        None,
        [
            "height" = N(0.8),
            "crown" = N(1.2),
            "trunk" = N(0.03),
            "clear" = N(0.0),
            "stems" = N(12.0),
            "habit" = T("mata"),
            "crown_shape" = T("cojín"),
            "foliage" = T("suculenta"),
            "levels" = N(2.0),
            "angle" = N(45.0),
            "density" = N(0.85),
            "clump" = N(0.1),
            "style" = T("dentado"),
        ]
    ),
    sp!(
        "Barrilla",
        "Matas y plantas de marisma",
        ("Hojas de barrilla", [150, 160, 120]),
        STEMS,
        None,
        None,
        [
            "height" = N(0.6),
            "crown" = N(1.0),
            "trunk" = N(0.03),
            "clear" = N(0.0),
            "stems" = N(12.0),
            "habit" = T("mata"),
            "crown_shape" = T("cojín"),
            "foliage" = T("hoja menuda"),
            "levels" = N(2.0),
            "angle" = N(50.0),
            "density" = N(0.85),
            "clump" = N(0.1),
            "style" = T("hojas"),
        ]
    ),
    // --- Grasses and rosettes ------------------------------------------
    sp!(
        "Esparto",
        "Gramíneas y suculentas",
        ("Hojas de esparto", [150, 160, 100]),
        STEMS,
        Some(("Espiga de esparto", [214, 200, 150])),
        None,
        [
            "height" = N(0.9),
            "crown" = N(1.0),
            "habit" = T("gramínea"),
            "foliage" = T("hojas largas"),
            "angle" = N(45.0),
            "droop" = N(0.5),
            "fronds" = N(50.0),
            "frond_length" = N(0.9),
            "bloom" = N(0.5),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Pennisetum",
        "Gramíneas y suculentas",
        ("Hojas de pennisetum", [140, 150, 90]),
        STEMS,
        Some(("Espiga de pennisetum", [222, 204, 186])),
        None,
        [
            "height" = N(1.0),
            "crown" = N(1.0),
            "habit" = T("gramínea"),
            "foliage" = T("hojas largas"),
            "angle" = N(50.0),
            "droop" = N(0.6),
            "fronds" = N(60.0),
            "frond_length" = N(0.9),
            "bloom" = N(0.8),
            "style" = T("radial"),
        ]
    ),
    sp!(
        "Pita",
        "Gramíneas y suculentas",
        ("Hojas de pita", [130, 155, 140]),
        STEMS,
        Some(("Flor de pita", [226, 206, 90])),
        None,
        [
            "height" = N(1.4),
            "crown" = N(2.2),
            "clear" = N(0.0),
            "habit" = T("roseta"),
            "foliage" = T("hojas largas"),
            "fronds" = N(28.0),
            "frond_length" = N(1.2),
            "style" = T("frondas"),
        ]
    ),
    sp!(
        "Yuca",
        "Gramíneas y suculentas",
        ("Hojas de yuca", [90, 120, 80]),
        BARK_PALE,
        None,
        None,
        [
            "height" = N(3.0),
            "crown" = N(2.0),
            "trunk" = N(0.15),
            "clear" = N(1.8),
            "stems" = N(3.0),
            "habit" = T("roseta"),
            "foliage" = T("hojas largas"),
            "fronds" = N(36.0),
            "frond_length" = N(0.7),
            "style" = T("frondas"),
        ]
    ),
];

/// Names of the species of the library, in order.
pub fn flora_names() -> impl Iterator<Item = &'static str> {
    FLORA_CATALOGUE.iter().map(|e| e.name)
}

fn paint(m: &mut Model, p: Paint) -> u64 {
    let found = m.materials().find(|x| x.name == p.0).map(|x| x.id);
    match found {
        Some(id) => id.0,
        None => m.add_material(p.0, "vegetation", p.1, CutPattern::Solid).0,
    }
}

/// The values of a species of the library (its materials made in `m` if
/// it has none of that name).
pub fn flora_values(m: &mut Model, e: &FloraEntry) -> Result<Values, ModelError> {
    // The materials first: the family's defaults need some.
    let leaves = paint(m, e.leaves);
    let bark = paint(m, e.bark);
    let flowers = paint(m, e.flowers.unwrap_or(e.leaves));
    let fruit = paint(m, e.fruit.unwrap_or(e.leaves));
    let f = m.system_family(VEGETATION_FAMILY)?;
    let mut values = m.family_defaults(f, ParamScope::Type)?;
    // Without a clear trunk of its own, a plant has none.
    values.insert("clear".into(), Value::Number(0.0));
    for (k, v) in e.values {
        values.insert(
            (*k).into(),
            match v {
                V::N(x) => Value::Number(*x),
                V::T(t) => Value::Text((*t).into()),
            },
        );
    }
    values.insert("leaves".into(), Value::Material(leaves));
    values.insert("bark".into(), Value::Material(bark));
    values.insert("flowers".into(), Value::Material(flowers));
    values.insert("fruits".into(), Value::Material(fruit));
    Ok(values)
}

/// Adds a species of the library as an object type (or finds the
/// project's type of that name).
pub fn add_flora_entry(m: &mut Model, name: &str) -> Result<Option<ObjectTypeId>, ModelError> {
    if let Some(t) = m.object_types().find(|t| t.name == name) {
        return Ok(Some(t.id));
    }
    let Some(e) = FLORA_CATALOGUE.iter().find(|e| e.name == name) else {
        return Ok(None);
    };
    let values = flora_values(m, e)?;
    let f = m.system_family(VEGETATION_FAMILY)?;
    let id = m.add_object_type(e.name, f, values)?;
    // Always the same `GlobalId`, so a species loaded in two projects is
    // one type.
    m.set_object_type_guid(
        id,
        crate::extended::stable_guid(&format!("flora/{}", e.name)),
    )?;
    Ok(Some(id))
}

/// A model with every species of the library, for the gallery's pictures.
pub fn flora_library() -> Model {
    let mut m = Model::new("Vegetación de la biblioteca");
    m.add_level("Planta baja", 0.0);
    for e in FLORA_CATALOGUE {
        add_flora_entry(&mut m, e.name).expect("the species of the library are valid");
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_species_loads_and_grows() {
        let m = flora_library();
        assert_eq!(m.object_types().count(), FLORA_CATALOGUE.len());
        for t in m.object_types() {
            assert!(m.type_grows(t.id), "{}", t.name);
            let look = m
                .plant_look_of(t.id, &Values::new(), 7)
                .unwrap_or_else(|| panic!("{}", t.name));
            let plant = archisim_core::flora::plant(&look.species, look.seed, look.variation);
            assert!(plant.height > 0.0, "{}", t.name);
            // Every choice of the species is one the family offers.
            let v = &t.values;
            for (k, x) in v {
                if let Value::Text(s) = x {
                    let def = m.family(t.family).unwrap().def();
                    let p = def
                        .params_of(ParamScope::Type)
                        .find(|p| &p.name == k)
                        .unwrap();
                    if let archisim_core::ParamKind::Choice(opts) = &p.kind {
                        assert!(opts.contains(s), "{}: {k} = {s}", t.name);
                    }
                }
            }
        }
    }
}
