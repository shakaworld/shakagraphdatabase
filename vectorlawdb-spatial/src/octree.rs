use nalgebra::Vector3;

const MAX_CASES_PER_NODE: usize = 8;

pub struct OctreeNode {
    center: Vector3<f32>,
    half_size: f32,
    cases: Vec<(Vector3<f32>, String)>,
    children: Option<Box<[OctreeNode; 8]>>,
    depth: usize,
}

impl OctreeNode {
    fn new(center: Vector3<f32>, half_size: f32, depth: usize) -> Self {
        Self {
            center,
            half_size,
            cases: Vec::new(),
            children: None,
            depth,
        }
    }

    fn contains(&self, point: &Vector3<f32>) -> bool {
        (point.x - self.center.x).abs() <= self.half_size &&
        (point.y - self.center.y).abs() <= self.half_size &&
        (point.z - self.center.z).abs() <= self.half_size
    }

    fn get_octant(&self, point: &Vector3<f32>) -> usize {
        let mut octant = 0;
        if point.x > self.center.x { octant |= 1; }
        if point.y < self.center.y { octant |= 2; }
        if point.z < self.center.z { octant |= 4; }
        octant
    }

    fn subdivide(&mut self, max_depth: usize) {
        if self.depth >= max_depth {
            return;
        }

        let quarter = self.half_size / 2.0;
        let mut children = Vec::with_capacity(8);

        for i in 0..8 {
            let offset = Vector3::new(
                if (i & 1) != 0 { quarter } else { -quarter },
                if (i & 2) != 0 { -quarter } else { quarter },
                if (i & 4) != 0 { -quarter } else { quarter },
            );

            let child_center = self.center + offset;
            children.push(OctreeNode::new(child_center, quarter, self.depth + 1));
        }

        // Redistribute cases
        for (position, case_id) in &self.cases {
            let octant = self.get_octant(position);
            children[octant].cases.push((*position, case_id.clone()));
        }

        self.cases.clear();
        self.children = Some(Box::new(children.try_into().unwrap()));
    }
}

pub struct Octree3D {
    root: OctreeNode,
    max_depth: usize,
    case_positions: std::collections::HashMap<String, Vector3<f32>>,
}

impl Octree3D {
    pub fn new(center: Vector3<f32>, size: f32, max_depth: usize) -> Self {
        Self {
            root: OctreeNode::new(center, size / 2.0, 0),
            max_depth,
            case_positions: std::collections::HashMap::new(),
        }
    }

    pub fn insert(&mut self, position: Vector3<f32>, case_id: String) {
        self.insert_recursive(&mut self.root, position, case_id.clone());
        self.case_positions.insert(case_id, position);
    }

    fn insert_recursive(&mut self, node: &mut OctreeNode, position: Vector3<f32>, case_id: String) {
        if !node.contains(&position) {
            return;
        }

        if node.children.is_none() {
            node.cases.push((position, case_id));

            if node.cases.len() > MAX_CASES_PER_NODE && node.depth < self.max_depth {
                node.subdivide(self.max_depth);
            }
        } else {
            let octant = node.get_octant(&position);
            if let Some(children) = &mut node.children {
                self.insert_recursive(&mut children[octant], position, case_id);
            }
        }
    }

    pub fn query_radius(&self, center: Vector3<f32>, radius: f32) -> Vec<String> {
        let mut results = Vec::new();
        self.query_radius_recursive(&self.root, &center, radius, &mut results);
        results
    }

    fn query_radius_recursive(
        &self,
        node: &OctreeNode,
        center: &Vector3<f32>,
        radius: f32,
        results: &mut Vec<String>,
    ) {
        // Sphere-box intersection test
        if !self.sphere_intersects_box(center, radius, &node.center, node.half_size) {
            return;
        }

        if node.children.is_none() {
            for (position, case_id) in &node.cases {
                if (position - center).magnitude() <= radius {
                    results.push(case_id.clone());
                }
            }
        } else if let Some(children) = &node.children {
            for child in children.iter() {
                self.query_radius_recursive(child, center, radius, results);
            }
        }
    }

    fn sphere_intersects_box(
        &self,
        sphere_center: &Vector3<f32>,
        sphere_radius: f32,
        box_center: &Vector3<f32>,
        box_half_size: f32,
    ) -> bool {
        let closest = Vector3::new(
            sphere_center.x.max(box_center.x - box_half_size).min(box_center.x + box_half_size),
            sphere_center.y.max(box_center.y - box_half_size).min(box_center.y + box_half_size),
            sphere_center.z.max(box_center.z - box_half_size).min(box_center.z + box_half_size),
        );

        (closest - sphere_center).magnitude() <= sphere_radius
    }

    pub fn get_case_position(&self, case_id: &str) -> Option<Vector3<f32>> {
        self.case_positions.get(case_id).copied()
    }
}
