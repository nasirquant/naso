//! Tensor module for Naso standard library
//!
//! Provides QTT-compliant tensor operations with linear type guarantees.

pub mod tensor;
pub mod ops;
pub mod polyhedral_lowering;

pub use tensor::*;
pub use ops::*;
pub use polyhedral_lowering::*;

use crate::core::prelude::*;

/// Marker trait for QTT quantity kinds
pub trait QttKind: Sized + 'static {
    const NAME: &'static str;
}

/// Proof-only zero-size shape marker
pub struct QttZero;
impl QttKind for QttZero {
    const NAME: &'static str = "QttZero";
}

/// Linear mutable buffer marker
pub struct QttLinear;
impl QttKind for QttLinear {
    const NAME: &'static str = "QttLinear";
}

/// Heap storage marker
pub struct QttHeap;
impl QttKind for QttHeap {
    const NAME: &'static str = "QttHeap";
}

/// Type-level quantity marker for tensor dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    Zero,
    Linear,
    Heap,
}

/// Trait for statically known tensor ranks
pub trait Rank: Sized + Copy + 'static {
    const VALUE: usize;
    type Prev: Rank;
}

impl Rank for () {
    const VALUE: usize = 0;
    type Prev = ();
}

pub struct Succ<R: Rank>(std::marker::PhantomData<R>);
impl<R: Rank> Rank for Succ<R> {
    const VALUE: usize = R::VALUE + 1;
    type Prev = R;
}

pub type Rank0 = ();
pub type Rank1 = Succ<Rank0>;
pub type Rank2 = Succ<Rank1>;
pub type Rank3 = Succ<Rank2>;
pub type Rank4 = Succ<Rank3>;
pub type Rank5 = Succ<Rank4>;
pub type Rank6 = Succ<Rank5>;
pub type Rank7 = Succ<Rank6>;
pub type Rank8 = Succ<Rank7>;

/// Type-level dimension list
pub trait Dims: Sized + 'static {
    const RANK: usize;
    const SIZES: &'static [usize];
    type Head: Dim;
    type Tail: Dims;
}

impl Dims for () {
    const RANK: usize = 0;
    const SIZES: &'static [usize] = &[];
    type Head = ();
    type Tail = ();
}

pub struct DimCons<D: Dim, DS: Dims>(std::marker::PhantomData<(D, DS)>);
impl<D: Dim, DS: Dims> Dims for DimCons<D, DS> {
    const RANK: usize = DS::RANK + 1;
    const SIZES: &'static [usize] = {
        const SIZE: usize = D::VALUE;
        const TAIL: &'static [usize] = DS::SIZES;
        &[SIZE]
    };
    type Head = D;
    type Tail = DS;
}

/// Type-level dimension
pub trait Dim: Sized + 'static {
    const VALUE: usize;
}

pub struct DimConst<const N: usize>;
impl<const N: usize> Dim for DimConst<N> {
    const VALUE: usize = N;
}

pub type Dim1 = DimConst<1>;
pub type Dim2 = DimConst<2>;
pub type Dim3 = DimConst<3>;
pub type Dim4 = DimConst<4>;
pub type Dim8 = DimConst<8>;
pub type Dim16 = DimConst<16>;
pub type Dim32 = DimConst<32>;
pub type Dim64 = DimConst<64>;
pub type Dim128 = DimConst<128>;
pub type Dim256 = DimConst<256>;
pub type Dim512 = DimConst<512>;
pub type Dim1024 = DimConst<1024>;

/// Shape trait for runtime shape validation
pub trait Shape: Sized + Clone + Send + Sync + 'static {
    fn rank(&self) -> usize;
    fn dims(&self) -> &[usize];
    fn num_elements(&self) -> usize;
    fn strides(&self) -> &[usize];
    fn is_contiguous(&self) -> bool;
}

/// Concrete shape representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcreteShape {
    dims: Vec<usize>,
    strides: Vec<usize>,
}

impl ConcreteShape {
    pub fn new(dims: Vec<usize>) -> Self {
        let mut strides = Vec::with_capacity(dims.len());
        let mut stride = 1;
        for &dim in dims.iter().rev() {
            strides.push(stride);
            stride *= dim;
        }
        strides.reverse();
        Self { dims, strides }
    }

    pub fn scalar() -> Self {
        Self::new(vec![])
    }

    pub fn vector(n: usize) -> Self {
        Self::new(vec![n])
    }

    pub fn matrix(rows: usize, cols: usize) -> Self {
        Self::new(vec![rows, cols])
    }
}

impl Shape for ConcreteShape {
    fn rank(&self) -> usize {
        self.dims.len()
    }

    fn dims(&self) -> &[usize] {
        &self.dims
    }

    fn num_elements(&self) -> usize {
        self.dims.iter().product()
    }

    fn strides(&self) -> &[usize] {
        &self.strides
    }

    fn is_contiguous(&self) -> bool {
        let mut expected_stride = 1;
        for (&dim, &stride) in self.dims.iter().rev().zip(self.strides.iter().rev()) {
            if stride != expected_stride {
                return false;
            }
            expected_stride *= dim;
        }
        true
    }
}

/// Broadcastable shape trait
pub trait Broadcastable: Shape {
    fn can_broadcast(&self, other: &Self) -> bool;
    fn broadcast_shape(&self, other: &Self) -> Option<Self>;
}

impl Broadcastable for ConcreteShape {
    fn can_broadcast(&self, other: &Self) -> bool {
        let max_rank = self.rank().max(other.rank());
        for i in 0..max_rank {
            let d1 = self.dims().get(self.rank().saturating_sub(max_rank - i));
            let d2 = other.dims().get(other.rank().saturating_sub(max_rank - i));
            match (d1, d2) {
                (Some(&a), Some(&b)) if a != b && a != 1 && b != 1 => return false,
                _ => {}
            }
        }
        true
    }

    fn broadcast_shape(&self, other: &Self) -> Option<Self> {
        if !self.can_broadcast(other) {
            return None;
        }
        let max_rank = self.rank().max(other.rank());
        let mut result_dims = Vec::with_capacity(max_rank);
        for i in 0..max_rank {
            let d1 = self.dims().get(self.rank().saturating_sub(max_rank - i)).copied().unwrap_or(1);
            let d2 = other.dims().get(other.rank().saturating_sub(max_rank - i)).copied().unwrap_or(1);
            result_dims.push(d1.max(d2));
        }
        Some(ConcreteShape::new(result_dims))
    }
}

/// Layout trait for memory layout
pub trait Layout: Sized + Copy + Clone + Send + Sync + 'static {
    fn element_offset(&self, shape: &dyn Shape, indices: &[usize]) -> usize;
}

/// Row-major (C-style) layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowMajor;
impl Layout for RowMajor {
    fn element_offset(&self, shape: &dyn Shape, indices: &[usize]) -> usize {
        let strides = shape.strides();
        indices.iter().zip(strides.iter()).map(|(&i, &s)| i * s).sum()
    }
}

/// Column-major (Fortran-style) layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColMajor;
impl Layout for ColMajor {
    fn element_offset(&self, shape: &dyn Shape, indices: &[usize]) -> usize {
        let dims = shape.dims();
        let mut offset = 0;
        let mut stride = 1;
        for (&dim, &idx) in dims.iter().zip(indices.iter()) {
            offset += idx * stride;
            stride *= dim;
        }
        offset
    }
}