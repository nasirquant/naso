const { test } = require('node:test');
const assert = require('node:assert');
const Parser = require('tree-sitter');
const Naso = require('../bindings/node');

test('basic parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        fn main() -> i32 {
            let x: i32 = 42;
            return x;
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.ok(tree.rootNode);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('quantum function parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        quantum fn bell_pair() -> (Qubit, Qubit) {
            let q1 = qalloc();
            let q2 = qalloc();
            hadamard(q1);
            cnot(q1, q2);
            (q1, q2)
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('tensor function parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        tensor fn matmul(A: Tensor<f64, [M, K]>, B: Tensor<f64, [K, N]>) -> Tensor<f64, [M, N]> {
            matmul(A, B)
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('polyhedral loop parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        polyhedral fn matmul_tiled(A: Tensor<f64, [M, K]>, B: Tensor<f64, [K, N]>, C: Tensor<f64, [M, N]>) {
            forall (i in 0..M, j in 0..N, k in 0..K) @schedule(parallel) {
                C[i, j] += A[i, k] * B[k, j];
            }
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('linear types parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        fn consume_linear(consume x: [1] i32) -> [1] i32 {
            x
        }

        fn inout_param(inout x: i32) {
            x = x + 1;
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('module imports parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        module test {
            use prelude::quantum;
            import {Qubit, hadamard} from prelude::quantum;
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('quantum intrinsics parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        quantum fn test_intrinsics() {
            let q = qalloc();
            hadamard(q);
            pauli_x(q);
            pauli_y(q);
            pauli_z(q);
            phase(1.57, q);
            let q2 = qalloc();
            cnot(q, q2);
            cz(q, q2);
            swap(q, q2);
            toffoli(q, q2, qalloc());
            measure(q);
            bell_pair();
            ghz_state(3);
            qft(q);
            grover_oracle(|q| true, q);
            barrier(q, q2);
            reset(q);
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('tensor intrinsics parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        tensor fn test_intrinsics() {
            let A = tensor![1.0, 2.0];
            let B = tensor![3.0, 4.0];
            matmul(A, B);
            matvec(A, B);
            outer_product(A, B);
            inner_product(A, B);
            tensor_product(A, B);
            contract(A, B, [0], [0]);
            transpose(A);
            permute(A, [1, 0]);
            reshape(A, [2]);
            slice(A, [0..1]);
            concatenate([A, B], 0);
            split(A, [1, 1], 0);
            einsum("i,i->", A, B);
            svd(A);
            qr(A);
            cholesky(A);
            lu(A);
            inv(A);
            det(A);
            trace(A);
            norm(A);
            normalize(A);
            softmax(A, 0);
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('type annotations parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        fn test_types(
            a: i32,
            b: f64,
            c: Qubit,
            d: Tensor<f64, [M, N]>,
            e: [1] i32,
            f: [0] i32,
            g: [*] i32,
            h: [5] i32,
            i: &i32,
            j: &mut i32,
            k: fn(i32) -> i32,
            l: (i32, f64),
            m: [i32; 10],
            n: MyStruct,
            o: Vec<i32>,
        ) {
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('complex expressions parsing', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        fn test_expr() {
            let x = 1 + 2 * 3;
            let y = a.b.c;
            let z = arr[0][1];
            let w = foo(1, 2, 3);
            let v = (1, 2.0, "three");
            let u = MyStruct { field: 42 };
            let t = [1, 2, 3];
            let s = x as f64;
            let r = |a, b| a + b;
            let q = match x { 0 => 1, _ => 0 };
            let p = circuit { hadamard(qalloc()); };
        }
    `;

    const tree = parser.parse(source);
    assert.ok(tree);
    assert.strictEqual(tree.rootNode.hasError(), false);
});

test('tree structure', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `fn add(a: i32, b: i32) -> i32 { a + b }`;

    const tree = parser.parse(source);
    const root = tree.rootNode;

    assert.strictEqual(root.type, 'source_file');

    const funcDef = root.namedChildren[0];
    assert.strictEqual(funcDef.type, 'function_definition');

    const signature = funcDef.namedChildren[0];
    assert.strictEqual(signature.type, 'function_signature');

    const name = signature.namedChildren[0];
    assert.strictEqual(name.type, 'identifier');
    assert.strictEqual(name.text, 'add');
});

test('query highlights', () => {
    const parser = new Parser();
    parser.setLanguage(Naso);

    const source = `
        fn main() -> i32 {
            let x: i32 = 42;
            return x;
        }
    `;

    const tree = parser.parse(source);
    const highlights = Naso.query(`
        (function_definition) @function
        (let_declaration) @variable
        (number_literal) @number
    `);

    const captures = highlights.captures(tree.rootNode);
    assert.ok(captures.length > 0);
});
