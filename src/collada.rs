use collaborate::v1_4::*;
use collaborate::v1_4::Mesh as ColladaMesh;
use polygon::geometry::mesh::Mesh as PolygonMesh;
use polygon::geometry::mesh::MeshBuilder;
use polygon::geometry::mesh::Vertex as PolygonVertex;
use polygon::math::*;
use std::fs::File;
use std::path::Path;

pub fn load_mesh<P: AsRef<Path>>(path: P) -> Result<PolygonMesh, &'static str> {
    let file = File::open(path).expect("Failed to open file");
    let document = Collada::read(file).expect("Failed to parse COLLADA document");

    for library in document.libraries().filter_map(Library::as_library_geometries) {
        let meshes = library.geometries()
            .filter_map(|geometry| geometry.geometric_element.as_mesh());
        for mesh in meshes {
            for polylist in mesh.primitives().filter_map(Primitive::as_polylist) {
                let mesh = process_polylist(mesh, polylist)?;

                // TODO: Support loading multiple meshes.
                return Ok(mesh);
            }
        }
    }

    Err("No meshes found in the document I guess")
}

pub fn process_polylist(mesh: &ColladaMesh, polylist: &Polylist) -> Result<PolygonMesh, &'static str> {
    let mut builder = MeshBuilder::new();
    let mut indices = Vec::new();

    for polygon in polylist {
        for vertex in &polygon {
            let mut position = None;
            let mut normal = None;
            let texcoord = Vec::new();

            // For each of the attributes in the vertex, find the correct input and then grab
            // the vertex data.
            for attribute in vertex {
                // Retrieve the raw data for each attribute that matches the attribute's offset.
                for input in polylist.inputs_for_offset(attribute.offset) {
                    // Handle the input based on its semantic.
                    match input.semantic.as_ref() {
                        // The "VERTEX" semantic means that this input indexes into all
                        // sources specified in the `vertices` member of the host mesh.
                        "VERTEX" => {
                            // We're assuming that the input refers to the mesh's `vertices`
                            // member. If that assumption is incorrect, we're going to produce
                            // the wrong mesh data.
                            assert_eq!(
                                mesh.vertices.id,
                                input.source.id(),
                                "Input targets a `Vertices` that doesn't belong to same mesh",
                            );

                            // Find the input that corresponds to the "POSITION" semantic. The
                            // COLLADA spec requires that there be one in a `<vertices>` element.
                            let input = mesh.vertices.inputs.iter()
                                .find(|input| input.semantic == "POSITION")
                                .expect("Vertices had no input with the \"POSITION\" semantic");

                            // Find the mesh source identified by the input's `source` within the
                            // parent `Mesh` object.
                            let source = mesh.find_source(input.source.id())
                                .expect("Didn't find a source with a matching ID in the parent mesh");

                            // Retrieve the source's accessor and raw float array. We only support
                            // using floats for position and normal source data, so we ignore
                            // any other type of array source.
                            let accessor = &source.common_accessor().expect("Source has no accessor");
                            let array = source.array.as_ref()
                                .and_then(Array::as_float_array)
                                .expect("Source wasn't a float array");

                            /// Use the accessor to get the position data for the current vertex.
                            let position_data = accessor.access(array.data.as_ref(), attribute.index);

                            // Use the `params` in the accesor to determine which elements in
                            // `normal_data` correspond to the normal's X, Y, and Z components.
                            let mut x = None;
                            let mut y = None;
                            let mut z = None;

                            for (param, &position_component) in accessor.params.iter().zip(position_data.iter()) {
                                match param.name.as_ref().map(String::as_str) {
                                    Some("X") => { x = Some(position_component); }
                                    Some("Y") => { y = Some(position_component); }
                                    Some("Z") => { z = Some(position_component); }

                                    // Ignore any unrecognized or unsupported names.
                                    _ => {}
                                }
                            }

                            position = Some(Point::new(
                                x.expect("Normal had no X component"),
                                y.expect("Normal had no Y component"),
                                z.expect("Normal had no Z component"),
                            ))
                        }

                        "NORMAL" => {
                            // Find the mesh source identified by the input's `source` within the
                            // parent `Mesh` object.
                            let source = mesh.find_source(input.source.id())
                                .expect("Didn't find a source with a matching ID in the parent mesh");

                            // Retrieve the source's accessor and raw float array. We only support
                            // using floats for position and normal source data, so we ignore
                            // any other type of array source.
                            let accessor = &source.common_accessor().expect("Source has no accessor");
                            let array = source.array.as_ref()
                                .and_then(Array::as_float_array)
                                .expect("Source wasn't a float array");

                            /// Use the accessor to get the normal data for the current vertex.
                            let normal_data = accessor.access(array.data.as_ref(), attribute.index);

                            // Use the `params` in the accesor to determine which elements in
                            // `normal_data` correspond to the normal's X, Y, and Z components.
                            let mut x = None;
                            let mut y = None;
                            let mut z = None;

                            for (param, &normal_component) in accessor.params.iter().zip(normal_data.iter()) {
                                match param.name.as_ref().map(String::as_str) {
                                    Some("X") => { x = Some(normal_component); }
                                    Some("Y") => { y = Some(normal_component); }
                                    Some("Z") => { z = Some(normal_component); }

                                    // Ignore any unrecognized or unsupported names.
                                    _ => {}
                                }
                            }

                            normal = Some(Vector3 {
                                x: x.expect("Normal had no X component"),
                                y: y.expect("Normal had no Y component"),
                                z: z.expect("Normal had no Z component"),
                            })
                        }

                        // Ignore any unknown semantics.
                        semantic @ _ => { println!("Ignoring unknown semantic {:?}", semantic); }
                    }
                }
            }

            let position = position.ok_or("Vertex missing position attribute")?;
            builder.add_vertex(PolygonVertex { position, normal, texcoord });
            let index = indices.len() as u32;
            indices.push(index);
        }
    }

    builder
        .set_indices(&*indices)
        .build()
        .map_err(|_| "Failed to build mesh")
}
