use anyhow::Result;
use std::io::BufRead;

//==================================================
//=== Object
//==================================================

#[derive(Debug)]
pub struct ObjectPool {
    pub indices: Vec<u16>,
    pub vertices: Vec<Vertex>,
    pub pool: Vec<ObjectData>,
}

impl ObjectPool {
    pub const CIRCLE: usize = 52;
    pub const CIRCLE_BORDER: usize = 53;
    pub const ROUNDED_RECTANGLE: usize = 54;
    pub const RECTANGLE_BORDER: usize = 55;
    pub const ROUNDED_RECTANGLE_BORDER: usize = 56;
    pub const RECTANGLE: usize = 57;

    fn print_loaded_objects(&self) {
        for (i, object) in self.pool.iter().enumerate() {
            println!("{}. {} loaded", i, object.name);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ObjectData {
    pub name: String,
    pub index_count: usize,
    pub index_offset: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3], // TODO! Change to [f32;2], Buffer Description Info needs to be updated!
    pub color: [f32; 3],
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: Default::default(),
            color: [1.0, 1.0, 1.0],
        }
    }
}

impl Vertex {
    pub fn from_xy(x: f32, y: f32) -> Self {
        Self {
            position: [x, y, 0.0],
            ..Default::default()
        }
    }
}

// #[derive(Clone, Default)]
// struct VertexColor {
//     pub name: String,
//     pub color: [f32; 3],
// }

/// Preload Object Pool
pub fn preload() -> Result<ObjectPool> {
    let pool = load_obj_files(&["chars", "shapes"]);

    if let Ok(pool) = &pool {
        pool.print_loaded_objects();
    }

    pool
}

/// Load .obj file without .mtl file
pub fn load_obj_files(obj_names: &[&str]) -> Result<ObjectPool> {
    let mut curr_line;

    /* 1. Load Vertices/Indices & Fill Object Pool*/

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut pool = Vec::new();

    let mut vertex = Vertex::default();
    let mut index;
    let mut object_index_offset = 0;
    let mut object_data = ObjectData::default();

    for obj_name in obj_names {
        let path = format!("res/obj/{}.obj", obj_name);
        let file = std::fs::File::open(path)?;
        for line in std::io::BufReader::new(file).lines() {
            curr_line = line?;

            if let Some(text) = curr_line.get(..2) {
                match text {
                    "o " => {
                        //"o object_name" -> "object_name"
                        if let Some(object_name) = curr_line.split(' ').next_back() {
                            // First Object -> Skip Save
                            if object_data.name.len() == 0 {
                                object_data.name = object_name.to_string();
                                continue;
                            }

                            // Save
                            pool.push(object_data.clone());
                            object_data.name = object_name.to_string();
                            object_data.index_offset += object_data.index_count;
                            object_data.index_count = 0;
                        }
                    }
                    "v " => {
                        //"v 0.000000 0.000000 -7.000000" -> [0.0, 0.0, -7.0]
                        for (i, value) in curr_line.split(' ').enumerate() {
                            if i == 0 {
                                continue;
                            }

                            if i > 3 {
                                break;
                            }

                            vertex.position[i - 1] = value.parse::<f32>()?;
                        }

                        vertices.push(vertex);
                    }
                    "f " => {
                        //"f 18 7 1" -> [18, 7, 1]
                        for (i, value) in curr_line.split(' ').enumerate() {
                            if i == 0 {
                                continue;
                            }

                            if i > 3 {
                                break;
                            }

                            index = value.parse::<u16>()? - 1;
                            indices.push(object_index_offset as u16 + index);
                        }

                        object_data.index_count += 3;
                    }
                    _ => (),
                }
            }
        }

        object_index_offset = vertices.len();
    }

    // Save Last Object
    pool.push(object_data);

    Ok(ObjectPool {
        indices,
        vertices,
        pool,
    })
}

/// Load a single .obj file with .mtl file
// pub fn load_obj_with_mtl(obj_name: &str) -> Result<ObjectPool> {
//     let mut curr_line;

//     /* 1. Load Colors */

//     let mut color_pool = Vec::new();

//     let mut vertex_color = VertexColor::default();

//     let path = format!("res/obj/{}.mtl", obj_name);
//     let file = std::fs::File::open(path)?;
//     for line in std::io::BufReader::new(file).lines() {
//         curr_line = line?;

//         if let Some(text) = curr_line.get(..2) {
//             match text {
//                 "ne" => {
//                     //newmtl Name -> Name
//                     if let Some(color_name) = curr_line.split(' ').next_back() {
//                         vertex_color.name = color_name.to_string();
//                     }
//                 }
//                 "Kd" => {
//                     //Kd 0.8 0.8 0.8 -> [0.8, 0.8, 0.8]
//                     for (i, value) in curr_line.split(' ').enumerate() {
//                         if i == 0 {
//                             continue;
//                         }

//                         if i > 3 {
//                             break;
//                         }

//                         vertex_color.color[i - 1] = value.parse::<f32>()?;
//                     }
//                 }
//                 "Ks" => {
//                     //Ks should come after Kd
//                     color_pool.push(vertex_color.clone());
//                 }
//                 _ => (),
//             }
//         }
//     }

//     /* 2. Load Vertices/Indices & Fill Object Pool*/

//     let mut vertices = Vec::new();
//     let mut indices = Vec::new();
//     let mut pool = Vec::new();

//     let mut vertex = Vertex::default();
//     let mut index;
//     let mut object_data = ObjectData::default();

//     let path = format!("res/obj/{}.obj", obj_name);
//     let file = std::fs::File::open(path)?;
//     for line in std::io::BufReader::new(file).lines() {
//         curr_line = line?;

//         if let Some(text) = curr_line.get(..2) {
//             match text {
//                 "o " => {
//                     //"o Test_Cube.001" -> "Test_Cube.001"
//                     if let Some(object_text) = curr_line.split(' ').next_back() {
//                         //"Test_Cube.001" -> "Test"
//                         if let Some(object_name) = object_text.split("_").next() {
//                             // First Object -> Skip Save
//                             if object_data.name.len() == 0 {
//                                 object_data.name = object_name.to_string();
//                                 continue;
//                             }

//                             // Save
//                             pool.push(object_data.clone());
//                             object_data.name = object_name.to_string();
//                             object_data.index_offset += object_data.index_count;
//                             object_data.index_count = 0;
//                         }
//                     }
//                 }
//                 "v " => {
//                     //"v 0.000000 0.000000 -7.000000" -> [0.0, 0.0, -7.0]
//                     for (i, value) in curr_line.split(' ').enumerate() {
//                         if i == 0 {
//                             continue;
//                         }

//                         if i > 3 {
//                             break;
//                         }

//                         vertex.position[i - 1] = value.parse::<f32>()?;
//                     }

//                     vertices.push(vertex);
//                 }
//                 "us" => {
//                     //"usemtl MaterialName" -> MaterialName
//                     if let Some(color_name) = curr_line.split(' ').next_back() {
//                         for color in &color_pool {
//                             if color.name == color_name {
//                                 vertex_color.color = color.color;
//                             }
//                         }
//                     }
//                 }
//                 "f " => {
//                     //"f 18 7 1" -> [18, 7, 1]
//                     for (i, value) in curr_line.split(' ').enumerate() {
//                         if i == 0 {
//                             continue;
//                         }

//                         if i > 3 {
//                             break;
//                         }

//                         index = value.parse::<u16>()? - 1;

//                         vertices[index as usize].color = vertex_color.color;

//                         indices.push(index);
//                     }

//                     object_data.index_count += 3;
//                 }
//                 _ => (),
//             }
//         }
//     }

//     // Save Last Object
//     pool.push(object_data);

//     Ok(ObjectPool {
//         indices,
//         vertices,
//         pool,
//     })
// }

//==================================================
//=== Text
//==================================================

// This maps the ASCII Char decimal number to the objects in char.obj
// Special Cases:
// [#]      [Draw]
// 255  ->  Nothing
// 254  ->  Space
// 253  ->  New Line
// 0    ->  Blank
#[rustfmt::skip]
pub const CHAR_OBJECT_POOL: [u8; 255] = [
    // 0 - 9 Unused Special Characters -> 255
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,

    // 10 New Line
    253,    // [LINE FEED]

    // 11 - 31 Unused Special Charachters -> 255
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 


    // 32 - 126 Space, Blank Characters & Normal Characters
    254,    // [SPACE]
    1,      // !
    2,      // "
    0,      // # -> Blank
    0,      // $ -> Blank
    0,      // % -> Blank
    0,      // & -> Blank
    3,      // '
    4,      // ( -> [
    5,      // ) -> ]
    6,      // *
    7,      // +
    10,     // ,
    9,      // -
    12,     // .
    11,     // /
    16,     // 0
    17,     // 1
    18,     // 2
    19,     // 3
    20,     // 4
    21,     // 5
    22,     // 6
    23,     // 7
    24,     // 8
    25,     // 9
    8,      // :
    0,      // ; -> Blank
    0,      // < -> Blank
    13,     // =
    0,      // > -> Blank
    14,     // ?
    0,      // @ -> Blank
    26,     // A
    27,     // B
    28,     // C
    29,     // D
    30,     // E
    31,     // F
    32,     // G
    33,     // H
    34,     // I
    35,     // J
    36,     // K
    37,     // L
    38,     // M
    39,     // N
    40,     // O
    41,     // P
    42,     // Q
    43,     // R
    44,     // S
    45,     // T
    46,     // U
    47,     // V
    48,     // W
    49,     // X
    50,     // Y
    51,     // Z
    4,      // [
    0,      // \ -> Blank
    5,      // ]
    0,      // ^ -> Blank
    15,     // _
    0,      // ` -> Blank
    26,     // a
    27,     // b
    28,     // c
    29,     // d
    30,     // e
    31,     // f
    32,     // g
    33,     // h
    34,     // i
    35,     // j
    36,     // k
    37,     // l
    38,     // m
    39,     // n
    40,     // o
    41,     // p
    42,     // q
    43,     // r
    44,     // s
    45,     // t
    46,     // u
    47,     // v
    48,     // w
    49,     // x
    50,     // y
    51,     // z
    4,      // { -> [
    0,      // | -> Blank
    5,      // } -> ]
    0,      // ~ -> Blank

    // 127 - 255 Empty Char
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255,
];

//==================================================
//=== Unit Testing
//==================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_multiple_objects() {
        let obj = load_obj_files(&["shapes"]).unwrap();

        assert_eq!(obj.pool.len(), 6);
    }
}
