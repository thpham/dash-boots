//! Tiny decoder-only transformer, inference only, no dependencies. Weights
//! come from tools/levelgen (LGM1 = f32, LGM2 = int8 with per-row scales,
//! dequantized at load) and are embedded in the wasm like the music: zero
//! fetch. Sampling is grammar-constrained: each step masks what
//! `Compiler::allowed`/`can_end` refuse, so every sample compiles; the masks
//! are returned (`Trace`) so GRPO trains on exactly what the game samples.

use super::compile::{CompiledLevel, Compiler};
use super::motif::{vocab, Theme};
use super::rng::Rng;

pub static WEIGHTS: &[u8] = include_bytes!("../../assets/levelgen.bin");

struct Layer {
    ln1_w: Vec<f32>,
    ln1_b: Vec<f32>,
    qkv_w: Vec<f32>,
    qkv_b: Vec<f32>,
    proj_w: Vec<f32>,
    proj_b: Vec<f32>,
    ln2_w: Vec<f32>,
    ln2_b: Vec<f32>,
    fc1_w: Vec<f32>,
    fc1_b: Vec<f32>,
    fc2_w: Vec<f32>,
    fc2_b: Vec<f32>,
}

pub struct Model {
    pub vocab: usize,
    pub ctx: usize,
    d: usize,
    n_head: usize,
    d_ff: usize,
    tok: Vec<f32>,
    pos: Vec<f32>,
    layers: Vec<Layer>,
    lnf_w: Vec<f32>,
    lnf_b: Vec<f32>,
}

struct Reader<'a> {
    b: &'a [u8],
    off: usize,
    int8: bool,
}

impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8], String> {
        let s = self.b.get(self.off..self.off + n).ok_or("levelgen weights: truncated")?;
        self.off += n;
        Ok(s)
    }
    fn f32s(&mut self, n: usize) -> Result<Vec<f32>, String> {
        Ok(self.bytes(4 * n)?.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
    }
    /// A 2-D weight (`rows × cols`), f32 or int8 + per-row scale.
    fn matrix(&mut self, rows: usize, cols: usize) -> Result<Vec<f32>, String> {
        if !self.int8 {
            return self.f32s(rows * cols);
        }
        let scales = self.f32s(rows)?;
        let q = self.bytes(rows * cols)?;
        Ok(q.iter().enumerate().map(|(i, v)| *v as i8 as f32 * scales[i / cols]).collect())
    }
}

impl Model {
    pub fn from_bytes(b: &[u8]) -> Result<Model, String> {
        let int8 = match b.get(..4) {
            Some(b"LGM1") => false,
            Some(b"LGM2") => true,
            _ => return Err("levelgen weights: bad magic".into()),
        };
        if b.len() < 28 {
            return Err("levelgen weights: truncated header".into());
        }
        let u = |i: usize| u32::from_le_bytes(b[4 + 4 * i..8 + 4 * i].try_into().unwrap()) as usize;
        let (vocab, ctx, d, n_layer, n_head, d_ff) = (u(0), u(1), u(2), u(3), u(4), u(5));
        if vocab != vocab::size() {
            return Err(format!("levelgen weights: vocab {vocab} != {} (retrain)", vocab::size()));
        }
        if n_head == 0 || d % n_head != 0 || ctx == 0 {
            return Err("levelgen weights: bad dimensions".into());
        }
        let mut r = Reader { b, off: 28, int8 };
        let tok = r.matrix(vocab, d)?;
        let pos = r.matrix(ctx, d)?;
        let mut layers = Vec::with_capacity(n_layer);
        for _ in 0..n_layer {
            layers.push(Layer {
                ln1_w: r.f32s(d)?,
                ln1_b: r.f32s(d)?,
                qkv_w: r.matrix(3 * d, d)?,
                qkv_b: r.f32s(3 * d)?,
                proj_w: r.matrix(d, d)?,
                proj_b: r.f32s(d)?,
                ln2_w: r.f32s(d)?,
                ln2_b: r.f32s(d)?,
                fc1_w: r.matrix(d_ff, d)?,
                fc1_b: r.f32s(d_ff)?,
                fc2_w: r.matrix(d, d_ff)?,
                fc2_b: r.f32s(d)?,
            });
        }
        let lnf_w = r.f32s(d)?;
        let lnf_b = r.f32s(d)?;
        if r.off != b.len() {
            return Err("levelgen weights: trailing bytes".into());
        }
        Ok(Model { vocab, ctx, d, n_head, d_ff, tok, pos, layers, lnf_w, lnf_b })
    }

    pub fn embedded() -> Result<Model, String> {
        Model::from_bytes(WEIGHTS)
    }

    pub fn n_params(&self) -> usize {
        let per = |l: &Layer| {
            4 * l.ln1_w.len()
                + l.qkv_w.len()
                + l.qkv_b.len()
                + l.proj_w.len()
                + l.proj_b.len()
                + l.fc1_w.len()
                + l.fc1_b.len()
                + l.fc2_w.len()
                + l.fc2_b.len()
        };
        self.tok.len() + self.pos.len() + self.layers.iter().map(per).sum::<usize>() + 2 * self.d
    }
}

/// One generation session (KV cache per layer).
pub struct Session<'m> {
    m: &'m Model,
    k: Vec<Vec<f32>>,
    v: Vec<Vec<f32>>,
    pub t: usize,
}

fn layer_norm(x: &[f32], w: &[f32], b: &[f32], out: &mut [f32]) {
    let n = x.len() as f32;
    let mean = x.iter().sum::<f32>() / n;
    let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n;
    let inv = 1.0 / (var + 1e-5).sqrt();
    for i in 0..x.len() {
        out[i] = (x[i] - mean) * inv * w[i] + b[i];
    }
}

/// out = W·x + b, W row-major [out, in] (torch Linear layout).
fn linear(w: &[f32], b: &[f32], x: &[f32], out: &mut [f32]) {
    let n_in = x.len();
    for (i, o) in out.iter_mut().enumerate() {
        let row = &w[i * n_in..(i + 1) * n_in];
        *o = b[i] + row.iter().zip(x).map(|(a, b)| a * b).sum::<f32>();
    }
}

fn gelu(x: f32) -> f32 {
    0.5 * x * (1.0 + (0.797_884_6 * (x + 0.044_715 * x * x * x)).tanh())
}

impl<'m> Session<'m> {
    pub fn new(m: &'m Model) -> Self {
        Session { m, k: vec![Vec::new(); m.layers.len()], v: vec![Vec::new(); m.layers.len()], t: 0 }
    }

    /// Feed one token, get the next-token logits.
    pub fn step(&mut self, token: u32) -> Vec<f32> {
        let m = self.m;
        let (d, nh) = (m.d, m.n_head);
        let hd = d / nh;
        let t = self.t.min(m.ctx - 1);
        let mut x: Vec<f32> = (0..d).map(|i| m.tok[token as usize * d + i] + m.pos[t * d + i]).collect();
        let (mut h, mut qkv, mut y, mut o) = (vec![0.0; d], vec![0.0; 3 * d], vec![0.0; d], vec![0.0; d]);
        let mut ff = vec![0.0; m.d_ff];
        let scale = 1.0 / (hd as f32).sqrt();
        for (li, l) in m.layers.iter().enumerate() {
            layer_norm(&x, &l.ln1_w, &l.ln1_b, &mut h);
            linear(&l.qkv_w, &l.qkv_b, &h, &mut qkv);
            self.k[li].extend_from_slice(&qkv[d..2 * d]);
            self.v[li].extend_from_slice(&qkv[2 * d..]);
            let (kc, vc) = (&self.k[li], &self.v[li]);
            let steps = kc.len() / d;
            for head in 0..nh {
                let q = &qkv[head * hd..(head + 1) * hd];
                let mut att: Vec<f32> = (0..steps)
                    .map(|s| {
                        let k = &kc[s * d + head * hd..s * d + (head + 1) * hd];
                        q.iter().zip(k).map(|(a, b)| a * b).sum::<f32>() * scale
                    })
                    .collect();
                let mx = att.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let mut sum = 0.0;
                for a in att.iter_mut() {
                    *a = (*a - mx).exp();
                    sum += *a;
                }
                let yh = &mut y[head * hd..(head + 1) * hd];
                yh.fill(0.0);
                for (s, a) in att.iter().enumerate() {
                    let v = &vc[s * d + head * hd..s * d + (head + 1) * hd];
                    for (yi, vi) in yh.iter_mut().zip(v) {
                        *yi += a / sum * vi;
                    }
                }
            }
            linear(&l.proj_w, &l.proj_b, &y, &mut o);
            for i in 0..d {
                x[i] += o[i];
            }
            layer_norm(&x, &l.ln2_w, &l.ln2_b, &mut h);
            linear(&l.fc1_w, &l.fc1_b, &h, &mut ff);
            for f in ff.iter_mut() {
                *f = gelu(*f);
            }
            linear(&l.fc2_w, &l.fc2_b, &ff, &mut o);
            for i in 0..d {
                x[i] += o[i];
            }
        }
        let xf = x.clone();
        layer_norm(&xf, &m.lnf_w, &m.lnf_b, &mut x);
        self.t += 1;
        (0..m.vocab).map(|v| m.tok[v * d..(v + 1) * d].iter().zip(&x).map(|(a, b)| a * b).sum()).collect()
    }
}

/// A sampled level with what GRPO needs to score and train on it.
pub struct Trace {
    /// `[theme, difficulty, motifs…, EOS]`.
    pub tokens: Vec<u32>,
    /// Legal-token bitmask at each generated position (`tokens[2..]`).
    pub masks: Vec<u64>,
    pub level: CompiledLevel,
}

/// Sample one level. `None` only if the grammar dead-ends (proved impossible,
/// kept as a guard) or the context runs out before a legal ending.
pub fn sample(m: &Model, theme: Theme, difficulty: u8, seed: u64, temperature: f32) -> Option<Trace> {
    let mut rng = Rng::new(seed);
    let mut s = Session::new(m);
    let mut c = Compiler::new(theme);
    let prefix = [vocab::theme_token(theme), vocab::diff_token(difficulty)];
    s.step(prefix[0]);
    let mut logits = s.step(prefix[1]);
    let mut tokens = prefix.to_vec();
    let mut masks = Vec::new();
    loop {
        let mut mask = 0u64;
        for t in 0..m.vocab as u32 {
            let ok = match vocab::token_motif(t) {
                Some(motif) => c.allowed(motif),
                None => t == vocab::EOS && c.can_end(),
            };
            if ok {
                mask |= 1 << t;
            }
        }
        if mask == 0 {
            return None;
        }
        // Max over *legal* tokens only: an illegal favourite must not underflow
        // every legal probability to zero.
        let mx = logits
            .iter()
            .enumerate()
            .filter(|(t, _)| mask & (1 << t) != 0)
            .map(|(_, l)| *l)
            .fold(f32::NEG_INFINITY, f32::max);
        let p: Vec<f32> = logits
            .iter()
            .enumerate()
            .map(|(t, l)| if mask & (1 << t) != 0 { ((l - mx) / temperature).exp() } else { 0.0 })
            .collect();
        let next = rng.weighted(&p)? as u32;
        masks.push(mask);
        tokens.push(next);
        if next == vocab::EOS {
            break;
        }
        c.push(vocab::token_motif(next).expect("masked to motifs"));
        if s.t >= m.ctx {
            return None;
        }
        logits = s.step(next);
    }
    Some(Trace { tokens, masks, level: c.finish() })
}
