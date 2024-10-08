use std::cmp::Ordering;
use std::time::{Duration, Instant};

struct Node<K: Ord + Clone, V> {
    key: K,
    value: V,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
    height: i32,
    timestamp: Instant,
}

pub struct AVLCache<K: Ord + Clone, V: Clone> {
    root: Option<Box<Node<K, V>>>,
    capacity: usize,
    size: usize,
    ttl: Duration,
}

impl<K: Ord + Clone, V> Node<K, V> {
    fn new(key: K, value: V) -> Self {
        Node {
            key,
            value,
            left: None,
            right: None,
            height: 1,
            timestamp: Instant::now(),
        }
    }

    fn height(&self) -> i32 {
        self.height
    }

    fn balance_factor(&self) -> i32 {
        let left_height = self.left.as_ref().map_or(0, |n| n.height());
        let right_height = self.right.as_ref().map_or(0, |n| n.height());
        left_height - right_height
    }

    fn update_height(&mut self) {
        let left_height = self.left.as_ref().map_or(0, |n| n.height());
        let right_height = self.right.as_ref().map_or(0, |n| n.height());
        self.height = 1 + std::cmp::max(left_height, right_height);
    }
}

impl<K: Ord + Clone, V: Clone> AVLCache<K, V> {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        AVLCache {
            root: None,
            capacity,
            size: 0,
            ttl,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        let now = Instant::now();
        if let Some(node) = self.get_node(key) {
            if now.duration_since(node.timestamp) < self.ttl {
                let value = node.value.clone();
                self.put(key.clone(), value.clone());
                Some(value)
            } else {
                self.remove(key);
                None
            }
        } else {
            None
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        let contains_key = self.contains_key(&key);
        if self.size == self.capacity && !contains_key {
            if let Some((min_key, _)) = self.min() {
                self.remove(&min_key);
            }
        }

        let new_root = {
            let old_root = self.root.take();
            self.insert_helper(old_root, key, value)
        };

        self.root = new_root;
        if !contains_key {
            self.size = self.size.min(self.capacity);
        }
    }

    fn insert_helper(&mut self, node: Option<Box<Node<K, V>>>, key: K, value: V) -> Option<Box<Node<K, V>>> {
        match node {
            None => {
                self.size += 1;
                Some(Box::new(Node::new(key, value)))
            }
            Some(mut node) => {
                match key.cmp(&node.key) {
                    Ordering::Equal => {
                        node.value = value;
                        node.timestamp = Instant::now();
                    }
                    Ordering::Less => {
                        let new_left = self.insert_helper(node.left.take(), key, value);
                        node.left = new_left;
                    }
                    Ordering::Greater => {
                        let new_right = self.insert_helper(node.right.take(), key, value);
                        node.right = new_right;
                    }
                }
                Some(self.balance(node))
            }
        }
    }
    
    fn contains_key(&self, key: &K) -> bool {
        self.get_node(key).is_some()
    }

    fn get_node(&self, key: &K) -> Option<&Node<K, V>> {
        let mut current = self.root.as_ref();
        while let Some(node) = current {
            match key.cmp(&node.key) {
                Ordering::Equal => return Some(node),
                Ordering::Less => current = node.left.as_ref(),
                Ordering::Greater => current = node.right.as_ref(),
            }
        }
        None
    }
    
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let (new_root, removed_value) = {
            let old_root = self.root.take();
            self.remove_recursive(old_root, key)
        };
        self.root = new_root;
        if removed_value.is_some() {
            self.size -= 1;
        }
        removed_value
    }

    fn remove_recursive(&mut self, node: Option<Box<Node<K, V>>>, key: &K) -> (Option<Box<Node<K, V>>>, Option<V>) {
        match node {
            None => (None, None),
            Some(mut node) => {
                match key.cmp(&node.key) {
                    Ordering::Equal => {
                        let value = node.value.clone();
                        match (node.left.take(), node.right.take()) {
                            (None, right) => (right, Some(value)),
                            (left, None) => (left, Some(value)),
                            (left, right) => {
                                let (new_right, min) = Self::remove_min(right.unwrap());
                                node.key = min.key;
                                node.value = min.value;
                                node.timestamp = min.timestamp;
                                node.left = left;
                                node.right = new_right;
                                (Some(self.balance(node)), Some(value))
                            }
                        }
                    }
                    Ordering::Less => {
                        let (new_left, removed_value) = self.remove_recursive(node.left.take(), key);
                        node.left = new_left;
                        (Some(self.balance(node)), removed_value)
                    }
                    Ordering::Greater => {
                        let (new_right, removed_value) = self.remove_recursive(node.right.take(), key);
                        node.right = new_right;
                        (Some(self.balance(node)), removed_value)
                    }
                }
            }
        }
    }

    fn remove_min(mut node: Box<Node<K, V>>) -> (Option<Box<Node<K, V>>>, Box<Node<K, V>>) {
        if node.left.is_none() {
            (node.right.take(), node)
        } else {
            let (new_left, min) = Self::remove_min(node.left.take().unwrap());
            node.left = new_left;
            (Some(node), min)
        }
    }

    fn balance(&mut self, mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        node.update_height();
        let balance = node.balance_factor();
        if balance > 1 {
            if node.left.as_ref().unwrap().balance_factor() < 0 {
                node.left = Some(self.rotate_left(node.left.take().unwrap()));
            }
            self.rotate_right(node)
        } else if balance < -1 {
            if node.right.as_ref().unwrap().balance_factor() > 0 {
                node.right = Some(self.rotate_right(node.right.take().unwrap()));
            }
            self.rotate_left(node)
        } else {
            node
        }
    }

    fn rotate_left(&mut self, mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut new_root = node.right.take().unwrap();
        node.right = new_root.left.take();
        node.update_height();
        new_root.left = Some(node);
        new_root.update_height();
        new_root
    }

    fn rotate_right(&mut self, mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut new_root = node.left.take().unwrap();
        node.left = new_root.right.take();
        node.update_height();
        new_root.right = Some(node);
        new_root.update_height();
        new_root
    }

    pub fn min(&self) -> Option<(K, V)> {
        fn min_node<K: Ord + Clone, V: Clone>(node: &Node<K, V>) -> (K, V) {
            match &node.left {
                None => (node.key.clone(), node.value.clone()),
                Some(left) => min_node(left),
            }
        }
        self.root.as_ref().map(|node| min_node(node))
    }

    pub fn clear(&mut self) {
        self.root = None;
        self.size = 0;
    }
}