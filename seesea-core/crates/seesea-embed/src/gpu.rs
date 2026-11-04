//! The encoder layers on a GPU (Vulkan through wgpu), for the `gpu` feature.
//!
//! Only the 24 transformer layers run here; token embedding and pooling stay on the CPU, so a
//! batch costs one upload of the embedded tokens and one download of the final hidden states.

use burn::backend::wgpu::WgpuDevice;
use burn::tensor::activation::gelu;
use burn::tensor::module::attention;
use burn::tensor::ops::AttentionModuleOptions;
use burn::tensor::{Device, FloatDType, Tensor, TensorData};

struct Linear {
    weight: Tensor<2>,
    bias: Tensor<2>,
}

impl Linear {
    fn forward(&self, x: Tensor<2>) -> Tensor<2> {
        if self.weight.dtype() == x.dtype() {
            return x.matmul(self.weight.clone().transpose()) + self.bias.clone();
        }
        // 16-bit weights: multiply in 16 bits, keep the activations and the sums in 32.
        x.cast(FloatDType::F16)
            .matmul(self.weight.clone().transpose())
            .cast(FloatDType::F32)
            + self.bias.clone()
    }
}

struct Norm {
    weight: Tensor<2>,
    bias: Tensor<2>,
    eps: f64,
}

impl Norm {
    fn forward(&self, x: Tensor<2>) -> Tensor<2> {
        let mean = x.clone().mean_dim(1);
        let centered = x - mean;
        let var = (centered.clone() * centered.clone()).mean_dim(1);
        centered / (var + self.eps).sqrt() * self.weight.clone() + self.bias.clone()
    }
}

struct Layer {
    q: Linear,
    k: Linear,
    v: Linear,
    out: Linear,
    attn_norm: Norm,
    up: Linear,
    down: Linear,
    out_norm: Norm,
}

pub struct GpuLayers {
    device: Device,
    /// Weights are kept as 16-bit floats (half the video memory and bandwidth) where the device allows it.
    half: bool,
    layers: Vec<Layer>,
    heads: usize,
    hidden: usize,
}

/// A layer's weights as plain f32 buffers: `(out, in)` row-major matrices (as stored in GGUF) and vectors.
pub struct LinearData {
    pub weight: Vec<f32>,
    pub bias: Vec<f32>,
    pub inputs: usize,
    pub outputs: usize,
}

pub struct NormData {
    pub weight: Vec<f32>,
    pub bias: Vec<f32>,
}

pub struct LayerData {
    pub q: LinearData,
    pub k: LinearData,
    pub v: LinearData,
    pub out: LinearData,
    pub attn_norm: NormData,
    pub up: LinearData,
    pub down: LinearData,
    pub out_norm: NormData,
}

impl GpuLayers {
    /// Open the default GPU. Returns `None` when there is no usable adapter (wgpu panics then).
    pub fn open() -> Option<Self> {
        let device =
            std::panic::catch_unwind(|| -> Device { WgpuDevice::default().into() }).ok()?;
        // Force adapter initialisation now, so a missing GPU shows up here and not mid-search.
        std::panic::catch_unwind(|| {
            let t = Tensor::<2>::from_data(TensorData::new(vec![1f32; 4], [2, 2]), &device);
            t.clone().matmul(t).into_data()
        })
        .ok()?;
        let half =
            std::env::var("SEESEA_GPU_F32").is_err() && device.supports_dtype(FloatDType::F16);
        Some(Self {
            device,
            half,
            layers: Vec::new(),
            heads: 0,
            hidden: 0,
        })
    }

    /// Set the model's shape; call before pushing layers.
    pub fn shaped(mut self, heads: usize, hidden: usize) -> Self {
        self.heads = heads;
        self.hidden = hidden;
        self
    }

    pub fn push(&mut self, eps: f64, l: LayerData) {
        let dev = &self.device;
        let half = self.half;
        let lin = |d: LinearData| Linear {
            weight: {
                let w =
                    Tensor::<2>::from_data(TensorData::new(d.weight, [d.outputs, d.inputs]), dev);
                if half { w.cast(FloatDType::F16) } else { w }
            },
            bias: Tensor::from_data(TensorData::new(d.bias, [1, d.outputs]), dev),
        };
        let h = self.hidden;
        let norm = |d: NormData| Norm {
            weight: Tensor::from_data(TensorData::new(d.weight, [1, h]), dev),
            bias: Tensor::from_data(TensorData::new(d.bias, [1, h]), dev),
            eps,
        };
        self.layers.push(Layer {
            q: lin(l.q),
            k: lin(l.k),
            v: lin(l.v),
            out: lin(l.out),
            attn_norm: norm(l.attn_norm),
            up: lin(l.up),
            down: lin(l.down),
            out_norm: norm(l.out_norm),
        });
    }

    /// Free the weights and hand the video memory back to the driver.
    pub fn release(&mut self) {
        self.layers.clear();
        self.device.memory_cleanup();
    }

    /// Run the layers over embedded tokens `(batch, seq, hidden)` and a 0/1 mask `(batch, seq)`.
    pub fn forward(&self, embedded: Vec<f32>, mask: &[u32], batch: usize, seq: usize) -> Vec<f32> {
        let dev = &self.device;
        let (h, heads) = (self.hidden, self.heads);
        let d = h / heads;
        let rows = batch * seq;
        let bias: Vec<f32> = mask
            .iter()
            .map(|&m| if m == 0 { -1e9 } else { 0.0 })
            .collect();
        let bias = Tensor::<4>::from_data(TensorData::new(bias, [batch, 1, 1, seq]), dev)
            .expand([batch, heads, seq, seq]);
        let split = |t: Tensor<2>| t.reshape([batch, seq, heads, d]).swap_dims(1, 2);

        let mut x = Tensor::<2>::from_data(TensorData::new(embedded, [rows, h]), dev);
        for l in &self.layers {
            let (q, k, v) = (
                split(l.q.forward(x.clone())),
                split(l.k.forward(x.clone())),
                split(l.v.forward(x.clone())),
            );
            // One fused kernel instead of matmul, softmax, matmul with the score matrix in memory.
            let ctx = attention(
                q,
                k,
                v,
                None,
                Some(bias.clone()),
                AttentionModuleOptions::default(),
            )
            .swap_dims(1, 2)
            .reshape([rows, h]);
            x = l.attn_norm.forward(l.out.forward(ctx) + x);
            let up = gelu(l.up.forward(x.clone()));
            x = l.out_norm.forward(l.down.forward(up) + x);
        }
        x.into_data()
            .try_into_vec::<f32>()
            .expect("hidden states are f32")
    }
}

impl Drop for GpuLayers {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: usize = 8;
    const FF: usize = 16;
    const HEADS: usize = 2;

    /// Deterministic pseudo-random values in -0.5..0.5.
    fn fill(n: usize, seed: u64) -> Vec<f32> {
        let mut s = seed | 1;
        (0..n)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                ((s >> 11) as f64 / (1u64 << 53) as f64 - 0.5) as f32
            })
            .collect()
    }

    fn linear(i: usize, o: usize, seed: u64) -> LinearData {
        LinearData {
            weight: fill(i * o, seed),
            bias: fill(o, seed + 100),
            inputs: i,
            outputs: o,
        }
    }

    fn norm(seed: u64) -> NormData {
        NormData {
            weight: fill(H, seed).iter().map(|v| 1.0 + v).collect(),
            bias: fill(H, seed + 7),
        }
    }

    fn layer() -> LayerData {
        LayerData {
            q: linear(H, H, 1),
            k: linear(H, H, 2),
            v: linear(H, H, 3),
            out: linear(H, H, 4),
            attn_norm: norm(5),
            up: linear(H, FF, 6),
            down: linear(FF, H, 7),
            out_norm: norm(8),
        }
    }

    // Plain-Rust reference of one layer, row by row.
    fn mm(x: &[f32], rows: usize, l: &LinearData) -> Vec<f32> {
        let mut y = vec![0f32; rows * l.outputs];
        for r in 0..rows {
            for o in 0..l.outputs {
                y[r * l.outputs + o] = l.bias[o]
                    + (0..l.inputs)
                        .map(|i| x[r * l.inputs + i] * l.weight[o * l.inputs + i])
                        .sum::<f32>();
            }
        }
        y
    }

    fn ln(x: &[f32], rows: usize, n: &NormData) -> Vec<f32> {
        let mut y = vec![0f32; x.len()];
        for r in 0..rows {
            let row = &x[r * H..(r + 1) * H];
            let mean = row.iter().sum::<f32>() / H as f32;
            let var = row.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / H as f32;
            for i in 0..H {
                y[r * H + i] = (row[i] - mean) / (var + 1e-5).sqrt() * n.weight[i] + n.bias[i];
            }
        }
        y
    }

    fn erf(x: f32) -> f32 {
        // Abramowitz & Stegun 7.1.26, absolute error below 1.5e-7.
        let t = 1.0 / (1.0 + 0.3275911 * x.abs());
        let poly = t
            * (0.254829592
                + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
        (1.0 - poly * (-x * x).exp()).copysign(x)
    }

    fn reference(x: &[f32], mask: &[u32], batch: usize, seq: usize, l: &LayerData) -> Vec<f32> {
        let rows = batch * seq;
        let (q, k, v) = (mm(x, rows, &l.q), mm(x, rows, &l.k), mm(x, rows, &l.v));
        let d = H / HEADS;
        let mut ctx = vec![0f32; rows * H];
        for b in 0..batch {
            for h in 0..HEADS {
                for i in 0..seq {
                    let sc: Vec<f32> = (0..seq)
                        .map(|j| {
                            let dot: f32 = (0..d)
                                .map(|c| {
                                    q[(b * seq + i) * H + h * d + c]
                                        * k[(b * seq + j) * H + h * d + c]
                                })
                                .sum();
                            if mask[b * seq + j] == 0 {
                                f32::NEG_INFINITY
                            } else {
                                dot / (d as f32).sqrt()
                            }
                        })
                        .collect();
                    let max = sc.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let e: Vec<f32> = sc.iter().map(|s| (s - max).exp()).collect();
                    let sum: f32 = e.iter().sum();
                    for c in 0..d {
                        ctx[(b * seq + i) * H + h * d + c] = (0..seq)
                            .map(|j| e[j] / sum * v[(b * seq + j) * H + h * d + c])
                            .sum();
                    }
                }
            }
        }
        let o = mm(&ctx, rows, &l.out);
        let res: Vec<f32> = o.iter().zip(x).map(|(a, b)| a + b).collect();
        let x1 = ln(&res, rows, &l.attn_norm);
        let up: Vec<f32> = mm(&x1, rows, &l.up)
            .iter()
            .map(|v| 0.5 * v * (1.0 + erf(v / 2f32.sqrt())))
            .collect();
        let down = mm(&up, rows, &l.down);
        let res: Vec<f32> = down.iter().zip(&x1).map(|(a, b)| a + b).collect();
        ln(&res, rows, &l.out_norm)
    }

    #[test]
    fn a_layer_on_the_gpu_matches_a_plain_rust_reference() {
        let Some(mut gpu) = GpuLayers::open().map(|g| g.shaped(HEADS, H)) else {
            eprintln!("no GPU adapter here; skipping");
            return;
        };
        gpu.push(1e-5, layer());
        let (batch, seq) = (2, 3);
        let x = fill(batch * seq * H, 42);
        // Second row has one padding slot at the end.
        let mask = [1, 1, 1, 1, 1, 0];
        let got = gpu.forward(x.clone(), &mask, batch, seq);
        let want = reference(&x, &mask, batch, seq, &layer());
        // Compare real tokens only; padded positions are never used.
        for r in 0..batch * seq {
            if mask[r] == 0 {
                continue;
            }
            for i in 0..H {
                let (a, b) = (got[r * H + i], want[r * H + i]);
                assert!(
                    (a - b).abs() < 2e-3,
                    "row {r} col {i}: gpu {a}, reference {b}"
                );
            }
        }
    }
}
