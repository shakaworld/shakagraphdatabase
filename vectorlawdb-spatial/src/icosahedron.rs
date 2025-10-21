use nalgebra::{Vector3, Matrix3};
use serde::{Serialize, Deserialize};

const PHI: f32 = 1.618033988749895; // Golden ratio

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriangularFace {
    pub face_id: usize,
    pub vertices: [Vector3<f32>; 3],
    pub centroid: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub area: f32,
}

impl TriangularFace {
    pub fn contains_point(&self, point: &Vector3<f32>) -> bool {
        // Barycentric coordinate test
        let v0 = self.vertices[0];
        let v1 = self.vertices[1];
        let v2 = self.vertices[2];

        let v0v1 = v1 - v0;
        let v0v2 = v2 - v0;
        let v0p = point - v0;

        let dot00 = v0v1.dot(&v0v1);
        let dot01 = v0v1.dot(&v0v2);
        let dot02 = v0v1.dot(&v0p);
        let dot11 = v0v2.dot(&v0v2);
        let dot12 = v0v2.dot(&v0p);

        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01 + 1e-10);
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

        u >= 0.0 && v >= 0.0 && (u + v) <= 1.0
    }
}

pub struct Icosahedron {
    radius: f32,
    vertices: Vec<Vector3<f32>>,
    pub faces: Vec<TriangularFace>,
    rotation_matrix: Matrix3<f32>,
}

impl Icosahedron {
    pub fn new(radius: f32) -> Self {
        let vertices = Self::generate_vertices(radius);
        let faces = Self::generate_faces(&vertices);

        Self {
            radius,
            vertices,
            faces,
            rotation_matrix: Matrix3::identity(),
        }
    }

    fn generate_vertices(radius: f32) -> Vec<Vector3<f32>> {
        let vertices = vec![
            Vector3::new(-1.0, PHI, 0.0),
            Vector3::new(1.0, PHI, 0.0),
            Vector3::new(-1.0, -PHI, 0.0),
            Vector3::new(1.0, -PHI, 0.0),
            Vector3::new(0.0, -1.0, PHI),
            Vector3::new(0.0, 1.0, PHI),
            Vector3::new(0.0, -1.0, -PHI),
            Vector3::new(0.0, 1.0, -PHI),
            Vector3::new(PHI, 0.0, -1.0),
            Vector3::new(PHI, 0.0, 1.0),
            Vector3::new(-PHI, 0.0, -1.0),
            Vector3::new(-PHI, 0.0, 1.0),
        ];

        // Normalize to sphere
        vertices.into_iter()
            .map(|v| v.normalize() * radius)
            .collect()
    }

    fn generate_faces(vertices: &[Vector3<f32>]) -> Vec<TriangularFace> {
        let indices: &[(usize, usize, usize)] = &[
            (0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11),
            (1, 5, 9), (5, 11, 4), (11, 10, 2), (10, 7, 6), (7, 1, 8),
            (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8), (3, 8, 9),
            (4, 9, 5), (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1),
        ];

        indices.iter().enumerate().map(|(face_id, &(i, j, k))| {
            let verts = [vertices[i], vertices[j], vertices[k]];
            let centroid = (verts[0] + verts[1] + verts[2]) / 3.0;

            let edge1 = verts[1] - verts[0];
            let edge2 = verts[2] - verts[0];
            let normal = edge1.cross(&edge2).normalize();
            let area = edge1.cross(&edge2).magnitude() * 0.5;

            TriangularFace {
                face_id,
                vertices: verts,
                centroid,
                normal,
                area,
            }
        }).collect()
    }

    pub fn rotate(&mut self, euler_angles: Vector3<f32>) {
        let (alpha, beta, gamma) = (euler_angles.x, euler_angles.y, euler_angles.z);

        let rx = Matrix3::new(
            1.0, 0.0, 0.0,
            0.0, alpha.cos(), -alpha.sin(),
            0.0, alpha.sin(), alpha.cos(),
        );

        let ry = Matrix3::new(
            beta.cos(), 0.0, beta.sin(),
            0.0, 1.0, 0.0,
            -beta.sin(), 0.0, beta.cos(),
        );

        let rz = Matrix3::new(
            gamma.cos(), -gamma.sin(), 0.0,
            gamma.sin(), gamma.cos(), 0.0,
            0.0, 0.0, 1.0,
        );

        let rotation = rz * ry * rx;

        // Apply rotation to all vertices
        for v in &mut self.vertices {
            *v = rotation * *v;
        }

        // Update faces
        for face in &mut self.faces {
            for v in &mut face.vertices {
                *v = rotation * *v;
            }
            face.centroid = rotation * face.centroid;
            face.normal = rotation * face.normal;
        }

        self.rotation_matrix = rotation * self.rotation_matrix;
    }

    pub fn find_face(&self, point: &Vector3<f32>) -> &TriangularFace {
        let normalized = point.normalize();

        // Find face with maximum dot product
        self.faces.iter()
            .max_by(|a, b| {
                let sim_a = normalized.dot(&a.centroid.normalize());
                let sim_b = normalized.dot(&b.centroid.normalize());
                sim_a.partial_cmp(&sim_b).unwrap()
            })
            .unwrap()
    }
}
