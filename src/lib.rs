//! Solvent is a dependency resolver library written in rust.
//!
//! Solvent helps you to resolve dependency orderings by building up a dependency
//! graph and then resolving the dependences of some target node in an order such
//! that each output depends only upon the previous outputs.
//!
//! It is currently quite simple, but is still useful.
//!
//! #Example
//!
//! ```rust
//! extern crate solvent;
//!
//! use solvent::DepGraph;
//!
//! fn main() {
//!     // Create a new empty DepGraph.  Must be `mut` or else it cannot
//!     // be used by the rest of the library.
//!     let mut depgraph: DepGraph = DepGraph::new();
//!
//!     // You can register a dependency like this.  Solvent will
//!     // automatically create nodes for any term it has not seen before.
//!     // This means 'b' depends on 'd'
//!     depgraph.register_dependency("b","d");
//!
//!     // You can also register multiple dependencies at once
//!     depgraph.register_dependencies("a",&["b","c","d"]);
//!     depgraph.register_dependencies("c",&["e"]);
//!
//!     // You must set a target to resolve the dependencies of that target
//!     depgraph.set_target("a");
//!
//!     // Iterate through each dependency.  The dependencies will be returned
//!     // in an order such that each output only depends on the previous
//!     // outputs (or nothing).  The target itself will be output last.
//!     for node in depgraph.satisfying_iter() {
//!         print!("{} ", node);
//!     }
//! }
//! ```
//!
//! The above will output:  `d b e c a`
//!
//! You can also mark some elements as already satisfied, and the
//! iterator will take that into account:
//!
//! ```ignore
//! depgraph.mark_as_satisfied(["e","c"]);
//! ```
//!
//! The algorithm is not deterministic, and may give a different answer each
//! time it is run.  Beware.
//!
//! Dependency cycles are detected and will cause a panic!()

#![crate_name = "solvent"]
#![crate_type = "lib"]

// Required for log and rustdoc:
#![feature(phase)]
#[phase(plugin, link)]

extern crate log;

use std::collections::{HashMap,HashSet};
use std::collections::hash_map::{Occupied,Vacant};
use std::iter::{Iterator};
#[allow(unused_imports)]
use std::task;

/// This is the dependency graph.  It must be mutable, as the
/// library uses internal properties in the graph to do its
/// calculations.
#[deriving(Clone)]
pub struct DepGraph {
    /// List of dependencies.  Key is the element, values are the
    /// other elements that the key element depends upon.
    pub dependencies: HashMap<String,HashSet<String>>,

    // (private) target we are trying to satisfy
    target: Option<String>,

    // (private) elements already satisfied
    satisfied: HashSet<String>,

    // (private) current path, for cycle detection
    curpath: HashSet<String>,
}

/// This iterates through the dependencies of the DepGraph's target
pub struct DepGraphIterator<'a> {
    depgraph: &'a mut DepGraph
}

/// This iterates through the dependencies of the DepGraph's target,
/// marking each element as satisfied as it is visited.
pub struct DepGraphSatisfyingIterator<'a> {
    depgraph: &'a mut DepGraph
}

impl DepGraph {

    /// Create an empty DepGraph.
    pub fn new() -> DepGraph
    {
        DepGraph {
            dependencies: HashMap::new(),
            target: None,
            curpath: HashSet::new(),
            satisfied: HashSet::new(),
        }
    }

    /// Add a dependency to a DepGraph.  The node does not need
    /// to pre-exist, nor do the dependency nodes.  But if the
    /// node does pre-exist, the depends_on will be added to its
    /// existing dependency list.
    pub fn register_dependency<'a>( &mut self,
                                node: &'a str,
                                depends_on: &'a str )
    {
        match self.dependencies.entry( String::from_str(node) ) {
            Vacant(entry) => {
                let mut deps = HashSet::with_capacity(1);
                deps.insert( String::from_str(depends_on) );
                entry.set( deps );
            },
            Occupied(mut entry) => {
                (*entry.get_mut()).insert(String::from_str(depends_on));
            },
        }
    }

    /// Add multiple dependencies of one node to a DepGraph.  The
    /// node does not need to pre-exist, nor do the dependency elements.
    /// But if the node does pre-exist, the depends_on will be added
    /// to its existing dependency list.
    pub fn register_dependencies<'a>( &mut self,
                                  node: &'a str,
                                  depends_on: &'a[&'a str] )
    {
        match self.dependencies.entry( String::from_str(node) ) {
            Vacant(entry) => {
                let mut deps = HashSet::with_capacity( depends_on.len() );
                for s in depends_on.iter() {
                    deps.insert( String::from_str(*s) );
                }
                entry.set( deps );
            },
            Occupied(mut entry) => {
                for s in depends_on.iter() {
                    (*entry.get_mut()).insert( String::from_str(*s) );
                }
            },
        }
    }

    /// This sets the target node.  Iteratators on the graph always
    /// get the dependencies of the target node.
    pub fn set_target<'a>( &mut self, target: &'a str )
    {
        self.target = Some(String::from_str(target));
    }

    /// This marks a node as satisfied.  Iterators will not output
    /// such nodes.
    pub fn mark_as_satisfied<'a>( &mut self,
                                   nodes: &'a[&'a str] )
    {
        for node in nodes.iter() {
            self.satisfied.insert(String::from_str(*node));
        }
    }

    fn get_next_dependency(&mut self, node: &String) -> String
    {
        if self.curpath.contains(node) {
            panic!("Circular dependency graph at {}",node);
        }
        self.curpath.insert(node.clone());

        let deplist = match self.dependencies.get(node) {
            None => return node.clone(),
            Some(deplist) => deplist.clone() // ouch
        };

        for n in deplist.iter() {
            if self.satisfied.contains(n) {
                continue;
            }
            return self.get_next_dependency(n);
        }
        // nodes dependencies are satisfied
        node.clone()
    }

    /// Get an iterator to iterate through the dependencies of
    /// the target node.  This iter() will loop forever if you dont
    /// mark nodes satisfied as you go.  If you want the iterator
    /// to take care of that, use satisfying_iter()
    pub fn iter<'a>(&'a mut self) -> DepGraphIterator<'a>
    {
        DepGraphIterator {
            depgraph: self
        }
    }

    /// Get an iterator to iterate through the dependencies of
    /// the target node, and also to mark those dependencies as
    /// satisfied as it goes.
    pub fn satisfying_iter<'a>(&'a mut self) -> DepGraphSatisfyingIterator<'a>
    {
        DepGraphSatisfyingIterator {
            depgraph: self
        }
    }
}

impl<'a> Iterator<String> for DepGraphIterator<'a> {
    /// Get next dependency.  This may panic!() if a cycle is detected.
    fn next(&mut self) -> Option<String>
    {
        let node = match self.depgraph.target {
            None => return None,
            Some(ref node) => node.clone()
        };
        if self.depgraph.satisfied.contains(&node) {
            return None;
        }
        self.depgraph.curpath.clear();
        Some(self.depgraph.get_next_dependency(&node))
    }
}

impl<'a> Iterator<String> for DepGraphSatisfyingIterator<'a> {
    /// Get next dependency.  This may panic!() if a cycle is detected.
    fn next(&mut self) -> Option<String>
    {
        let node = match self.depgraph.target {
            None => return None,
            Some(ref node) => node.clone()
        };
        if self.depgraph.satisfied.contains(&node) {
            return None;
        }
        self.depgraph.curpath.clear();
        let next = self.depgraph.get_next_dependency(&node);
        self.depgraph.mark_as_satisfied(&[next.as_slice()]);
        Some(next)
    }
}

#[test]
fn solvent_test_branching() {
    let mut depgraph: DepGraph = DepGraph::new();

    depgraph.register_dependencies("a",&["b","c","d"]);
    depgraph.register_dependency("b","d");
    depgraph.register_dependencies("c",&["e","m","g"]);
    depgraph.register_dependency("e","f");
    depgraph.register_dependency("g","h");
    depgraph.register_dependency("h","i");
    depgraph.register_dependencies("i",&["j","k"]);
    depgraph.register_dependencies("k",&["l","m"]);
    depgraph.register_dependency("m","n");

    depgraph.set_target("a");

    let mut results: Vec<String> = Vec::new();

    loop {
        // detect infinite looping bugs
        assert!(results.len() < 30);

        let node = match depgraph.iter().next() {
            Some(x) => x,
            None => break,
        };
        depgraph.mark_as_satisfied(&[node.as_slice()]);

        // Check that all of that nodes dependencies have already been output
        let deps: Option<&HashSet<String>> = depgraph.dependencies.get(&node);
        if deps.is_some() {
            for dep in deps.unwrap().iter() {
                assert!( results.contains(dep) );
            }
        }

        results.push(node.clone());
    }
}

#[test]
fn solvent_test_updating_dependencies() {
    let mut depgraph: DepGraph = DepGraph::new();

    depgraph.register_dependencies("a",&["b","c"]);
    depgraph.register_dependency("a","d");
    assert!(depgraph.dependencies.get("a").unwrap().contains("b"));
    assert!(depgraph.dependencies.get("a").unwrap().contains("c"));
    assert!(depgraph.dependencies.get("a").unwrap().contains("d"));
}

#[test]
fn solvent_test_satisfying() {
    let mut depgraph: DepGraph = DepGraph::new();

    depgraph.register_dependencies("a",&["b","c","d"]);
    depgraph.register_dependency("b","d");
    depgraph.register_dependencies("c",&["e"]);

    depgraph.set_target("a");

    // To get past the borrow checker, we make a copy of the dependencies.
    // This is a test, and performance is not important here.
    let dependency_copy = depgraph.dependencies.clone();

    let mut results: Vec<String> = Vec::new();

    for node in depgraph.satisfying_iter() {
        // detect infinite looping bugs
        assert!(results.len() < 30);

        // Check that all of that nodes dependencies have already been output
        let deps: Option<&HashSet<String>> = dependency_copy.get(&node);
        if deps.is_some() {
            for dep in deps.unwrap().iter() {
                assert!( results.contains(dep) );
            }
        }

        results.push(node);
    }
}

#[test]
#[should_fail]
fn solvent_test_circular() {

    let mut depgraph: DepGraph = DepGraph::new();
    depgraph.register_dependency("a","b");
    depgraph.register_dependency("b","c");
    depgraph.register_dependency("c","a");
    depgraph.set_target("a");

    let mut results: Vec<String> = Vec::new();

    loop {
        // Detect infinite looping bugs
        // (Since this test should fail, we cause a success here)
        if results.len() >= 30 { break; }

        let node = match depgraph.iter().next() {
            Some(x) => x,
            None => break,
        };
        depgraph.mark_as_satisfied(&[node.as_slice()]);
        results.push(node);
    }
}

#[test]
fn solvent_test_satisfied_stoppage() {

    let mut depgraph: DepGraph = DepGraph::new();
    depgraph.register_dependencies("superconn", &[]);
    depgraph.register_dependencies("owneruser", &["superconn"]);
    depgraph.register_dependencies("appuser", &["superconn"]);
    depgraph.register_dependencies("database", &["owneruser"]);
    depgraph.register_dependencies("ownerconn", &["database","owneruser"]);
    depgraph.register_dependencies("adminconn", &["database"]);
    depgraph.register_dependencies("extensions", &["database","adminconn"]);
    depgraph.register_dependencies("schema_table", &["database","ownerconn"]);
    depgraph.register_dependencies("schemas", &["ownerconn","extensions","schema_table","appuser"]);
    depgraph.register_dependencies("appconn", &["database","appuser","schemas"]);

    depgraph.set_target("appconn");
    depgraph.mark_as_satisfied(&["owneruser","appuser"]);

    let mut results: Vec<String> = Vec::new();

    loop {
        assert!(results.len() < 30);

        let node = match depgraph.iter().next() {
            Some(x) => x,
            None => break,
        };
        depgraph.mark_as_satisfied(&[node.as_slice()]);
        results.push(node);
    }
    assert!( !results.contains(&String::from_str("superconn")) );
}

