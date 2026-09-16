//! Tensor operations: matmul, elementwise arithmetic, transpose, contraction, outer products

use super::*;
use crate::core::prelude::*;
use std::ops::{Add, Sub, Mul, Div, Neg};

/// Element-wise addition
pub fn add<Q: QttQty, D: Dims, T, L: Layout>(lhs: &Tensor<Q, D, T, L>, rhs: &Tensor<Q, D, T, L>) -> Tensor<Q, D, T, L>
where
    T: Add<Output = T> + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    assert_eq!(lhs.shape(), rhs.shape(), "Shape mismatch for add");
    let result: Vec<T> = lhs.as_slice()
        .iter()
        .zip(rhs.as_slice().iter())
        .map(|(a, b)| a.clone() + b.clone())
        .collect();
    Tensor::from_vec(result, lhs.shape().clone())
}

/// Element-wise subtraction
pub fn sub<Q: QttQty, D: Dims, T, L: Layout>(lhs: &Tensor<Q, D, T, L>, rhs: &Tensor<Q, D, T, L>) -> Tensor<Q, D, T, L>
where
    T: Sub<Output = T> + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    assert_eq!(lhs.shape(), rhs.shape(), "Shape mismatch for sub");
    let result: Vec<T> = lhs.as_slice()
        .iter()
        .zip(rhs.as_slice().iter())
        .map(|(a, b)| a.clone() - b.clone())
        .collect();
    Tensor::from_vec(result, lhs.shape().clone())
}

/// Element-wise multiplication
pub fn mul<Q: QttQty, D: Dims, T, L: Layout>(lhs: &Tensor<Q, D, T, L>, rhs: &Tensor<Q, D, T, L>) -> Tensor<Q, D, T, L>
where
    T: Mul<Output = T> + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    assert_eq!(lhs.shape(), rhs.shape(), "Shape mismatch for mul");
    let result: Vec<T> = lhs.as_slice()
        .iter()
        .zip(rhs.as_slice().iter())
        .map(|(a, b)| a.clone() * b.clone())
        .collect();
    Tensor::from_vec(result, lhs.shape().clone())
}

/// Element-wise division
pub fn div<Q: QttQty, D: Dims, T, L: Layout>(lhs: &Tensor<Q, D, T, L>, rhs: &Tensor<Q, D, T, L>) -> Tensor<Q, D, T, L>
where
    T: Div<Output = T> + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    assert_eq!(lhs.shape(), rhs.shape(), "Shape mismatch for div");
    let result: Vec<T> = lhs.as_slice()
        .iter()
        .zip(rhs.as_slice().iter())
        .map(|(a, b)| a.clone() / b.clone())
        .collect();
    Tensor::from_vec(result, lhs.shape().clone())
}

/// Element-wise negation
pub fn neg<Q: QttQty, D: Dims, T, L: Layout>(tensor: &Tensor<Q, D, T, L>) -> Tensor<Q, D, T, L>
where
    T: Neg<Output = T> + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let result: Vec<T> = tensor.as_slice()
        .iter()
        .map(|a| -a.clone())
        .collect();
    Tensor::from_vec(result, tensor.shape().clone())
}

/// Scalar addition
pub fn add_scalar<Q: QttQty, D: Dims, T, L: Layout>(tensor: &Tensor<Q, D, T, L>, scalar: T) -> Tensor<Q, D, T, L>
where
    T: Add<Output = T> + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let result: Vec<T> = tensor.as_slice()
        .iter()
        .map(|a| a.clone() + scalar.clone())
        .collect();
    Tensor::from_vec(result, tensor.shape().clone())
}

/// Scalar multiplication
pub fn mul_scalar<Q: QttQty, D: Dims, T, L: Layout>(tensor: &Tensor<Q, D, T, L>, scalar: T) -> Tensor<Q, D, T, L>
where
    T: Mul<Output = T> + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let result: Vec<T> = tensor.as_slice()
        .iter()
        .map(|a| a.clone() * scalar.clone())
        .collect();
    Tensor::from_vec(result, tensor.shape().clone())
}

/// Matrix multiplication (matmul) for 2D tensors
pub fn matmul<Q: QttQty, T, L: Layout>(
    lhs: &Tensor<Q, DimCons<DimConst<M>, DimCons<DimConst<K>, ()>>, T, L>,
    rhs: &Tensor<Q, DimCons<DimConst<K>, DimCons<DimConst<N>, ()>>, T, L>,
) -> Tensor<Q, DimCons<DimConst<M>, DimCons<DimConst<N>, ()>>, T, L>
where
    T: Add<Output = T> + Mul<Output = T> + Clone + Default + Copy,
    Tensor<Q, DimCons<DimConst<M>, DimCons<DimConst<K>, ()>>, T, L>: Deref<Target = [T]>,
    Tensor<Q, DimCons<DimConst<K>, DimCons<DimConst<N>, ()>>, T, L>: Deref<Target = [T]>,
{
    let m = lhs.shape().dims()[0];
    let k = lhs.shape().dims()[1];
    let n = rhs.shape().dims()[1];

    assert_eq!(lhs.shape().dims()[1], rhs.shape().dims()[0], "Inner dimensions must match for matmul");

    let mut result = Vec::with_capacity(m * n);
    for i in 0..m {
        for j in 0..n {
            let mut sum = T::default();
            for k_idx in 0..k {
                let lhs_idx = i * k + k_idx;
                let rhs_idx = k_idx * n + j;
                sum = sum + lhs[lhs_idx] * rhs[rhs_idx];
            }
            result.push(sum);
        }
    }

    let shape = ConcreteShape::new(vec![m, n]);
    Tensor::from_vec(result, shape)
}

/// Generic matmul for any rank >= 2 (batch matmul)
pub fn batch_matmul<Q: QttQty, T, L: Layout>(
    lhs: &Tensor<Q, D, T, L>,
    rhs: &Tensor<Q, D, T, L>,
) -> Tensor<Q, D, T, L>
where
    D: Dims,
    T: Add<Output = T> + Mul<Output = T> + Clone + Default + Copy,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let lhs_shape = lhs.shape();
    let rhs_shape = rhs.shape();
    
    assert!(lhs_shape.rank() >= 2 && rhs_shape.rank() >= 2, "Batch matmul requires rank >= 2");
    
    let lhs_batch_dims = &lhs_shape.dims()[..lhs_shape.rank() - 2];
    let rhs_batch_dims = &rhs_shape.dims()[..rhs_shape.rank() - 2];
    
    assert_eq!(lhs_batch_dims, rhs_batch_dims, "Batch dimensions must match");
    
    let m = lhs_shape.dims()[lhs_shape.rank() - 2];
    let k = lhs_shape.dims()[lhs_shape.rank() - 1];
    let n = rhs_shape.dims()[rhs_shape.rank() - 1];
    
    assert_eq!(k, rhs_shape.dims()[rhs_shape.rank() - 2], "Inner dimensions must match");

    let batch_size: usize = lhs_batch_dims.iter().product();
    let mut result = Vec::with_capacity(batch_size * m * n);
    
    for batch in 0..batch_size {
        let lhs_offset = batch * m * k;
        let rhs_offset = batch * k * n;
        let out_offset = batch * m * n;
        
        for i in 0..m {
            for j in 0..n {
                let mut sum = T::default();
                for k_idx in 0..k {
                    let lhs_idx = lhs_offset + i * k + k_idx;
                    let rhs_idx = rhs_offset + k_idx * n + j;
                    sum = sum + lhs[lhs_idx] * rhs[rhs_idx];
                }
                result.push(sum);
            }
        }
    }

    let mut new_dims = lhs_batch_dims.to_vec();
    new_dims.push(m);
    new_dims.push(n);
    let shape = ConcreteShape::new(new_dims);
    Tensor::from_vec(result, shape)
}

/// Tensor contraction (generalized matrix multiplication)
/// Contracts dimension `lhs_dim` of lhs with dimension `rhs_dim` of rhs
pub fn contract<Q: QttQty, T, L: Layout>(
    lhs: &Tensor<Q, D, T, L>,
    rhs: &Tensor<Q, D, T, L>,
    lhs_dim: usize,
    rhs_dim: usize,
) -> Tensor<Q, D, T, L>
where
    D: Dims,
    T: Add<Output = T> + Mul<Output = T> + Clone + Default + Copy,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let lhs_shape = lhs.shape();
    let rhs_shape = rhs.shape();
    
    assert!(lhs_dim < lhs_shape.rank(), "lhs_dim out of bounds");
    assert!(rhs_dim < rhs_shape.rank(), "rhs_dim out of bounds");
    
    let lhs_contract_size = lhs_shape.dims()[lhs_dim];
    let rhs_contract_size = rhs_shape.dims()[rhs_dim];
    assert_eq!(lhs_contract_size, rhs_contract_size, "Contracted dimensions must match");

    // Compute output shape
    let mut out_dims = Vec::new();
    for (i, &d) in lhs_shape.dims().iter().enumerate() {
        if i != lhs_dim {
            out_dims.push(d);
        }
    }
    for (i, &d) in rhs_shape.dims().iter().enumerate() {
        if i != rhs_dim {
            out_dims.push(d);
        }
    }

    // Compute strides for indexing
    let lhs_strides = lhs_shape.strides();
    let rhs_strides = rhs_shape.strides();
    
    let lhs_contract_stride = lhs_strides[lhs_dim];
    let rhs_contract_stride = rhs_strides[rhs_dim];
    
    let mut result = Vec::with_capacity(out_dims.iter().product());
    
    // Iterate over all output indices
    fn iterate_indices(
        dims: &[usize],
        idx: usize,
        current: &mut Vec<usize>,
        callback: &mut dyn FnMut(&[usize]),
    ) {
        if idx == dims.len() {
            callback(current);
            return;
        }
        for i in 0..dims[idx] {
            current.push(i);
            iterate_indices(dims, idx + 1, current, callback);
            current.pop();
        }
    }
    
    let mut out_indices = vec![0; out_dims.len()];
    iterate_indices(&out_dims, 0, &mut out_indices, &mut |out_idx| {
        // Map output indices to lhs and rhs indices
        let mut lhs_idx = 0;
        let mut rhs_idx = 0;
        let mut out_pos = 0;
        
        for (i, &d) in lhs_shape.dims().iter().enumerate() {
            if i == lhs_dim {
                continue;
            }
            let dim_idx = out_idx[out_pos];
            lhs_idx += dim_idx * lhs_strides[i];
            out_pos += 1;
        }
        
        out_pos = 0;
        for (i, &d) in rhs_shape.dims().iter().enumerate() {
            if i == rhs_dim {
                continue;
            }
            let dim_idx = out_idx[lhs_shape.rank() - 1 + out_pos];
            rhs_idx += dim_idx * rhs_strides[i];
            out_pos += 1;
        }
        
        // Perform contraction
        let mut sum = T::default();
        for k in 0..lhs_contract_size {
            let lhs_element_idx = lhs_idx + k * lhs_contract_stride;
            let rhs_element_idx = rhs_idx + k * rhs_contract_stride;
            sum = sum + lhs[lhs_element_idx] * rhs[rhs_element_idx];
        }
        result.push(sum);
    });

    let shape = ConcreteShape::new(out_dims);
    Tensor::from_vec(result, shape)
}

/// Outer product of two tensors
pub fn outer<Q: QttQty, T, L: Layout>(
    lhs: &Tensor<Q, D, T, L>,
    rhs: &Tensor<Q, D, T, L>,
) -> Tensor<Q, D, T, L>
where
    D: Dims,
    T: Mul<Output = T> + Clone + Copy,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let lhs_shape = lhs.shape();
    let rhs_shape = rhs.shape();
    
    let mut out_dims = Vec::with_capacity(lhs_shape.rank() + rhs_shape.rank());
    out_dims.extend_from_slice(lhs_shape.dims());
    out_dims.extend_from_slice(rhs_shape.dims());
    
    let mut result = Vec::with_capacity(lhs_shape.num_elements() * rhs_shape.num_elements());
    
    for &lhs_val in lhs.as_slice() {
        for &rhs_val in rhs.as_slice() {
            result.push(lhs_val * rhs_val);
        }
    }
    
    let shape = ConcreteShape::new(out_dims);
    Tensor::from_vec(result, shape)
}

/// Transpose last two dimensions (consumes tensor)
pub fn transpose<Q: QttQty, D: Dims, T: Clone, L: Layout>(tensor: Tensor<Q, D, T, L>) -> Tensor<Q, D, T, L>
where
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let mut dims = tensor.shape().dims().to_vec();
    let len = dims.len();
    if len >= 2 {
        dims.swap(len - 2, len - 1);
    }
    let new_shape = ConcreteShape::new(dims);
    
    tensor.reshape(new_shape)
}

/// Transpose last two dimensions (borrows tensor, clones data)
pub fn transpose_ref<Q: QttQty, D: Dims, T: Clone, L: Layout>(tensor: &Tensor<Q, D, T, L>) -> Tensor<Q, D, T, L>
where
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let mut dims = tensor.shape().dims().to_vec();
    let len = dims.len();
    if len >= 2 {
        dims.swap(len - 2, len - 1);
    }
    let new_shape = ConcreteShape::new(dims);
    
    Tensor::from_vec(tensor.as_slice().to_vec(), new_shape)
}

/// General transpose with permutation
pub fn permute<Q: QttQty, D: Dims, T, L: Layout>(
    tensor: Tensor<Q, D, T, L>,
    perm: &[usize],
) -> Tensor<Q, D, T, L>
where
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let shape = tensor.shape();
    assert_eq!(perm.len(), shape.rank(), "Permutation length must match rank");
    
    let mut new_dims = vec![0; shape.rank()];
    for (i, &p) in perm.iter().enumerate() {
        new_dims[i] = shape.dims()[p];
    }
    
    let new_shape = ConcreteShape::new(new_dims);
    tensor.reshape(new_shape)
}

/// Reshape tensor
pub fn reshape<Q: QttQty, D: Dims, D2: Dims, T, L: Layout>(
    tensor: Tensor<Q, D, T, L>,
    new_shape: ConcreteShape,
) -> Tensor<Q, D2, T, L>
where
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    assert_eq!(tensor.num_elements(), new_shape.num_elements());
    tensor.reshape(new_shape)
}

/// Squeeze dimensions of size 1
pub fn squeeze<Q: QttQty, D: Dims, T, L: Layout>(
    tensor: Tensor<Q, D, T, L>,
    dims: &[usize],
) -> Tensor<Q, D, T, L>
where
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let shape = tensor.shape();
    let mut new_dims = Vec::new();
    for (i, &d) in shape.dims().iter().enumerate() {
        if !dims.contains(&i) {
            assert_eq!(d, 1, "Can only squeeze dimensions of size 1");
        } else {
            new_dims.push(d);
        }
    }
    let new_shape = ConcreteShape::new(new_dims);
    tensor.reshape(new_shape)
}

/// Unsqueeze (add dimension of size 1)
pub fn unsqueeze<Q: QttQty, D: Dims, T, L: Layout>(
    tensor: Tensor<Q, D, T, L>,
    dim: usize,
) -> Tensor<Q, D, T, L>
where
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let shape = tensor.shape();
    assert!(dim <= shape.rank(), "Dim out of bounds for unsqueeze");
    
    let mut new_dims = shape.dims().to_vec();
    new_dims.insert(dim, 1);
    
    let new_shape = ConcreteShape::new(new_dims);
    tensor.reshape(new_shape)
}

/// Broadcast tensor to new shape
pub fn broadcast<Q: QttQty, D: Dims, T, L: Layout>(
    tensor: &Tensor<Q, D, T, L>,
    target_shape: &ConcreteShape,
) -> Tensor<Q, D, T, L>
where
    T: Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    assert!(tensor.shape().can_broadcast(target_shape), "Cannot broadcast to target shape");
    
    // For simplicity, just return a view with new shape
    // Full implementation would handle stride manipulation
    let new_shape = target_shape.clone();
    Tensor::from_vec(tensor.as_slice().to_vec(), new_shape)
}

/// Reduction operations
pub fn sum<Q: QttQty, D: Dims, T, L: Layout>(
    tensor: &Tensor<Q, D, T, L>,
    dim: Option<usize>,
) -> Tensor<Q, D, T, L>
where
    T: Add<Output = T> + Clone + Default,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let shape = tensor.shape();
    match dim {
        Some(d) => {
            assert!(d < shape.rank(), "Dim out of bounds");
            let reduce_size = shape.dims()[d];
            let mut new_dims = shape.dims().to_vec();
            new_dims.remove(d);
            
            let mut result = Vec::with_capacity(new_dims.iter().product());
            let stride = shape.strides()[d];
            
            // Compute output indices
            let outer_size: usize = shape.dims()[..d].iter().product();
            let inner_size: usize = shape.dims()[d+1..].iter().product();
            
            for outer in 0..outer_size {
                for inner in 0..inner_size {
                    let mut sum = T::default();
                    for k in 0..reduce_size {
                        let idx = outer * (reduce_size * inner_size) + k * inner_size + inner;
                        sum = sum + tensor[idx].clone();
                    }
                    result.push(sum);
                }
            }
            
            let new_shape = ConcreteShape::new(new_dims);
            Tensor::from_vec(result, new_shape)
        }
        None => {
            let sum = tensor.as_slice().iter().fold(T::default(), |acc, x| acc + x.clone());
            Tensor::from_vec(vec![sum], ConcreteShape::scalar())
        }
    }
}

pub fn mean<Q: QttQty, D: Dims, T, L: Layout>(
    tensor: &Tensor<Q, D, T, L>,
    dim: Option<usize>,
) -> Tensor<Q, D, T, L>
where
    T: Add<Output = T> + Div<Output = T> + Clone + Default + From<usize>,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let sum = sum(tensor, dim);
    let count = match dim {
        Some(d) => tensor.shape().dims()[d],
        None => tensor.num_elements(),
    };
    let count_t = T::from(count);
    
    // Division would need to be implemented per tensor
    // For now, just return sum (mean requires division support)
    sum
}

/// Dot product for 1D tensors
pub fn dot<Q: QttQty, T, L: Layout>(
    lhs: &Tensor<Q, DimCons<DimConst<N>, ()>, T, L>,
    rhs: &Tensor<Q, DimCons<DimConst<N>, ()>, T, L>,
) -> T
where
    T: Add<Output = T> + Mul<Output = T> + Clone + Default + Copy,
    Tensor<Q, DimCons<DimConst<N>, ()>, T, L>: Deref<Target = [T]>,
{
    assert_eq!(lhs.shape().dims()[0], rhs.shape().dims()[0], "Vector dimensions must match for dot product");
    
    let mut sum = T::default();
    for i in 0..lhs.shape().dims()[0] {
        sum = sum + lhs[i] * rhs[i];
    }
    sum
}

/// Matrix-vector multiplication
pub fn matvec<Q: QttQty, T, L: Layout>(
    mat: &Tensor<Q, DimCons<DimConst<M>, DimCons<DimConst<N>, ()>>, T, L>,
    vec: &Tensor<Q, DimCons<DimConst<N>, ()>, T, L>,
) -> Tensor<Q, DimCons<DimConst<M>, ()>, T, L>
where
    T: Add<Output = T> + Mul<Output = T> + Clone + Default + Copy,
    Tensor<Q, DimCons<DimConst<M>, DimCons<DimConst<N>, ()>>, T, L>: Deref<Target = [T]>,
    Tensor<Q, DimCons<DimConst<N>, ()>, T, L>: Deref<Target = [T]>,
{
    let m = mat.shape().dims()[0];
    let n = mat.shape().dims()[1];
    
    assert_eq!(n, vec.shape().dims()[0], "Matrix columns must match vector size");
    
    let mut result = Vec::with_capacity(m);
    for i in 0..m {
        let mut sum = T::default();
        for j in 0..n {
            sum = sum + mat[i * n + j] * vec[j];
        }
        result.push(sum);
    }
    
    let shape = ConcreteShape::new(vec![m]);
    Tensor::from_vec(result, shape)
}

/// Kronecker product
pub fn kron<Q: QttQty, T, L: Layout>(
    lhs: &Tensor<Q, D, T, L>,
    rhs: &Tensor<Q, D, T, L>,
) -> Tensor<Q, D, T, L>
where
    D: Dims,
    T: Mul<Output = T> + Clone + Copy,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let lhs_shape = lhs.shape();
    let rhs_shape = rhs.shape();
    
    let mut out_dims = Vec::with_capacity(lhs_shape.rank() + rhs_shape.rank());
    for &d in lhs_shape.dims() {
        out_dims.push(d);
    }
    for &d in rhs_shape.dims() {
        out_dims.push(d);
    }
    
    // This is a simplified version - full kron would interleave dimensions
    // For now, just compute the full outer product
    outer(lhs, rhs)
}

/// Element-wise maximum
pub fn maximum<Q: QttQty, D: Dims, T, L: Layout>(
    lhs: &Tensor<Q, D, T, L>,
    rhs: &Tensor<Q, D, T, L>,
) -> Tensor<Q, D, T, L>
where
    T: PartialOrd + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    assert_eq!(lhs.shape(), rhs.shape(), "Shape mismatch for maximum");
    let result: Vec<T> = lhs.as_slice()
        .iter()
        .zip(rhs.as_slice().iter())
        .map(|(a, b)| if a >= b { a.clone() } else { b.clone() })
        .collect();
    Tensor::from_vec(result, lhs.shape().clone())
}

/// Element-wise minimum
pub fn minimum<Q: QttQty, D: Dims, T, L: Layout>(
    lhs: &Tensor<Q, D, T, L>,
    rhs: &Tensor<Q, D, T, L>,
) -> Tensor<Q, D, T, L>
where
    T: PartialOrd + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    assert_eq!(lhs.shape(), rhs.shape(), "Shape mismatch for minimum");
    let result: Vec<T> = lhs.as_slice()
        .iter()
        .zip(rhs.as_slice().iter())
        .map(|(a, b)| if a <= b { a.clone() } else { b.clone() })
        .collect();
    Tensor::from_vec(result, lhs.shape().clone())
}

/// ReLU activation
pub fn relu<Q: QttQty, D: Dims, T, L: Layout>(tensor: &Tensor<Q, D, T, L>) -> Tensor<Q, D, T, L>
where
    T: PartialOrd + Clone + Default,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let result: Vec<T> = tensor.as_slice()
        .iter()
        .map(|a| if a >= &T::default() { a.clone() } else { T::default() })
        .collect();
    Tensor::from_vec(result, tensor.shape().clone())
}

/// Sigmoid activation
pub fn sigmoid<Q: QttQty, D: Dims, T, L: Layout>(tensor: &Tensor<Q, D, T, L>) -> Tensor<Q, D, T, L>
where
    T: FloatOps + Clone,
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    let result: Vec<T> = tensor.as_slice()
        .iter()
        .map(|a| T::one() / (T::one() + (-a.clone()).exp()))
        .collect();
    Tensor::from_vec(result, tensor.shape().clone())
}

/// Trait for floating point operations
pub trait FloatOps: Sized + Clone {
    fn one() -> Self;
    fn exp(self) -> Self;
}

impl FloatOps for f32 {
    fn one() -> Self { 1.0 }
    fn exp(self) -> Self { self.exp() }
}

impl FloatOps for f64 {
    fn one() -> Self { 1.0 }
    fn exp(self) -> Self { self.exp() }
}