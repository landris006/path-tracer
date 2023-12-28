use core::f32;

use cgmath::Vector3;

use crate::model::Triangle;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Node {
    min_corner: [f32; 3],
    left_child_index: u32,
    max_corner: [f32; 3],
    triangle_count: u32,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            min_corner: [f32::MAX; 3],
            left_child_index: 0,
            max_corner: [f32::MIN; 3],
            triangle_count: 0,
        }
    }
}

pub struct Bvh {
    pub nodes: Vec<Node>,
    pub triangle_indices: Vec<u32>,
    nodes_used: usize,
}

impl Bvh {
    pub fn from_triangles(triangles: &[Triangle]) -> Self {
        let nodes = vec![Node::default(); triangles.len() * 2 - 1];
        let triangle_indices = triangles
            .iter()
            .enumerate()
            .map(|(i, _)| i as u32)
            .collect::<Vec<_>>();

        let mut new_bvh = Self {
            nodes,
            nodes_used: 0,
            triangle_indices,
        };

        let root = new_bvh.nodes.get_mut(0).unwrap();
        root.left_child_index = 0;
        root.triangle_count = triangles.len() as u32;
        new_bvh.increment_nodes_used();

        new_bvh.update_bounds(0, triangles);
        new_bvh.subdivide(0, triangles);

        new_bvh
    }

    fn update_bounds(&mut self, node_index: usize, triangles: &[Triangle]) {
        let node = self
            .nodes
            .get_mut(node_index)
            .expect("Node index out of bounds");

        (0..node.triangle_count).for_each(|i| {
            let triangle = triangles
                .get(self.triangle_indices[(node.left_child_index + i) as usize] as usize)
                .expect("Triangle index out of bounds");

            triangle.vertices().iter().for_each(|vertex| {
                node.min_corner[0] = node.min_corner[0].min(vertex[0]);
                node.min_corner[1] = node.min_corner[1].min(vertex[1]);
                node.min_corner[2] = node.min_corner[2].min(vertex[2]);

                node.max_corner[0] = node.max_corner[0].max(vertex[0]);
                node.max_corner[1] = node.max_corner[1].max(vertex[1]);
                node.max_corner[2] = node.max_corner[2].max(vertex[2]);
            });
        })
    }

    fn subdivide(&mut self, node_index: usize, triangles: &[Triangle]) {
        let node = *self
            .nodes
            .get(node_index)
            .expect("Node index out of bounds");

        if node.triangle_count <= 2 {
            return;
        }

        let extent = Vector3::from(node.max_corner) - Vector3::from(node.min_corner);
        let mut axis = 0;
        if extent[1] > extent[axis] {
            axis = 1;
        }
        if extent[2] > extent[axis] {
            axis = 2;
        }

        let split_position = node.min_corner[axis] + extent[axis] / 2.0;

        let mut i = node.left_child_index;
        let mut j = node.left_child_index + node.triangle_count - 1;

        while i < j {
            if triangles[self.triangle_indices[i as usize] as usize]
                .vertices()
                .iter()
                .any(|v| v[axis] < split_position)
            {
                i += 1;
            } else {
                self.triangle_indices.swap(i as usize, j as usize);
                j -= 1;
            }
        }

        let left_count = i - node.left_child_index;
        if left_count == 0 || left_count == node.triangle_count {
            return;
        }

        let left_child_index = self.nodes_used as u32;
        self.increment_nodes_used();
        let right_child_index = self.nodes_used as u32;
        self.increment_nodes_used();

        self.nodes
            .get_mut(left_child_index as usize)
            .expect("Node index out of bounds")
            .left_child_index = node.left_child_index;
        self.nodes
            .get_mut(left_child_index as usize)
            .expect("Node index out of bounds")
            .triangle_count = left_count;

        self.nodes
            .get_mut(right_child_index as usize)
            .expect("Node index out of bounds")
            .left_child_index = i;
        self.nodes
            .get_mut(right_child_index as usize)
            .expect("Node index out of bounds")
            .triangle_count = node.triangle_count - left_count;

        let node = self.nodes.get_mut(node_index).unwrap();
        node.left_child_index = left_child_index;
        node.triangle_count = 0;

        self.update_bounds(left_child_index as usize, triangles);
        self.update_bounds(right_child_index as usize, triangles);
        self.subdivide(left_child_index as usize, triangles);
        self.subdivide(right_child_index as usize, triangles);
    }

    fn increment_nodes_used(&mut self) {
        self.nodes_used += 1;
    }
}

