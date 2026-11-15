//! The stack one nesting level costs, in a debug build
//!
//! serde's derived `Serialize` / `Deserialize` of an enum keep a slot of the serializer's
//! `Result<(), Error>` for every variant arm, so the frame of one level of a many-variant
//! recursive enum grows with the size of the codec's error. With a 32-byte error a left-deep
//! chain of an enum shaped like ALICE-SDF's `SdfNode` (about 140 struct variants) took 75,104
//! bytes of stack per level in debug, against bincode 1.3.3's 30,032 (aarch64): a chain bincode
//! 1 encoded on a 2 MiB thread overflowed the stack here. The error the encoder and decoder carry
//! is now one pointer wide.
//!
//! `Node` below has 128 struct variants and one recursive variant. The tests encode and decode
//! a left-deep chain of `DEPTH` levels on a thread with a `STACK` byte stack, in a child process
//! (a stack overflow aborts the process; the parent reads its exit status), so an overflow is a
//! failure of this test, not of the whole test binary.
//!
//! Author: Moroya Sakamoto
#![cfg(feature = "serde")]

use std::process::Command;

use alice_serial::bincode1;
use serde::{Deserialize, Serialize};

/// Levels of the chain (the depth limit is raised to admit it)
const DEPTH: usize = 40;
/// Bytes of stack for the encoding / decoding thread
const STACK: usize = 2 << 20;

macro_rules! node {
    ($($v:ident),*) => {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum Node {
            Leaf { r: f32 },
            Union { a: Box<Node>, b: Box<Node>, k: f32 },
            $($v { x: f32, y: f32, z: f32, w: f32 },)*
        }
    };
}

node!(
    V00, V01, V02, V03, V04, V05, V06, V07, V08, V09, V10, V11, V12, V13, V14, V15, V16, V17, V18,
    V19, V20, V21, V22, V23, V24, V25, V26, V27, V28, V29, V30, V31, V32, V33, V34, V35, V36, V37,
    V38, V39, V40, V41, V42, V43, V44, V45, V46, V47, V48, V49, V50, V51, V52, V53, V54, V55, V56,
    V57, V58, V59, V60, V61, V62, V63, V64, V65, V66, V67, V68, V69, V70, V71, V72, V73, V74, V75,
    V76, V77, V78, V79, V80, V81, V82, V83, V84, V85, V86, V87, V88, V89, V90, V91, V92, V93, V94,
    V95, V96, V97, V98, V99, W00, W01, W02, W03, W04, W05, W06, W07, W08, W09, W10, W11, W12, W13,
    W14, W15, W16, W17, W18, W19, W20, W21, W22, W23, W24, W25, W26, W27
);

/// A left-deep chain of `n` unions
fn chain(n: usize) -> Node {
    let mut node = Node::Leaf { r: 1.0 };
    let mut x = 0.0f32;
    for _ in 0..n {
        x += 1.0;
        node = Node::Union {
            a: Box::new(node),
            b: Box::new(Node::V00 {
                x,
                y: 0.0,
                z: 0.0,
                w: 0.0,
            }),
            k: 0.5,
        };
    }
    node
}

/// Runs `f` on a thread with `STACK` bytes of stack
fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(f)
        .expect("spawns")
        .join()
        .expect("joins");
}

/// The child: encodes and decodes the chain on the small stack (run by the parent test)
#[test]
#[ignore = "run by a_deep_chain_round_trips_on_a_small_stack in a child process"]
fn child_round_trip_on_a_small_stack() {
    let Some(depth) = std::env::var("ALICE_SERIAL_STACK_CHILD")
        .ok()
        .and_then(|d| d.parse::<usize>().ok())
    else {
        return;
    };
    // the chain is built and dropped on a large stack: only the codec runs on the small one
    let built = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || chain(depth))
        .unwrap()
        .join()
        .unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    on_small_stack(move || {
        let bytes = bincode1::to_vec(&built, usize::MAX).expect("encodes");
        let back: Node =
            bincode1::from_slice_with_depth(&bytes, bytes.len(), 4 * depth).expect("decodes");
        tx.send((built, back, bytes.len())).unwrap();
    });
    let (built, back, len) = rx.recv().unwrap();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            assert!(back == built, "round trip");
            assert!(len > 0);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn a_deep_chain_round_trips_on_a_small_stack() {
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "child_round_trip_on_a_small_stack",
            "--ignored",
            "--nocapture",
        ])
        .env("ALICE_SERIAL_STACK_CHILD", DEPTH.to_string())
        .output()
        .expect("runs the child");
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "a chain of {DEPTH} levels did not round-trip on a {STACK}-byte stack: {}",
        text.lines()
            .filter(|l| l.contains("overflow") || l.contains("panicked"))
            .collect::<Vec<_>>()
            .join(" / ")
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("1 passed"),
        "the child ran"
    );
}
