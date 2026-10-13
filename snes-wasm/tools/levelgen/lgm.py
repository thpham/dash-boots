"""Shared pieces of the level-model pipeline: vocabulary, model, corpus,
LGM1 (f32) / LGM2 (int8, per-row scales) export. Layout must match
wasm/src/levelgen/model.rs."""

import os
import struct

import torch
import torch.nn as nn
import torch.nn.functional as F

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
CRATE = os.path.join(ROOT, 'wasm')
ASSET = os.path.join(CRATE, 'assets', 'levelgen.bin')

CTX = 80
D, N_LAYER, N_HEAD, D_FF = 96, 4, 4, 384
PAD, EOS = 0, 1


def vocab():
    """Token names in id order (written by `just levelgen-corpus`)."""
    with open(os.path.join(HERE, 'vocab.txt')) as f:
        return f.read().split()


class Block(nn.Module):
    def __init__(self):
        super().__init__()
        self.ln1 = nn.LayerNorm(D)
        self.qkv = nn.Linear(D, 3 * D)
        self.proj = nn.Linear(D, D)
        self.ln2 = nn.LayerNorm(D)
        self.fc1 = nn.Linear(D, D_FF)
        self.fc2 = nn.Linear(D_FF, D)

    def forward(self, x):
        b, t, _ = x.shape
        q, k, v = self.qkv(self.ln1(x)).split(D, dim=2)
        sh = lambda z: z.view(b, t, N_HEAD, D // N_HEAD).transpose(1, 2)
        y = F.scaled_dot_product_attention(sh(q), sh(k), sh(v), is_causal=True)
        x = x + self.proj(y.transpose(1, 2).reshape(b, t, D))
        return x + self.fc2(F.gelu(self.fc1(self.ln2(x)), approximate='tanh'))


class Model(nn.Module):
    def __init__(self, v):
        super().__init__()
        self.tok = nn.Embedding(v, D)
        self.pos = nn.Embedding(CTX, D)
        self.blocks = nn.ModuleList(Block() for _ in range(N_LAYER))
        self.lnf = nn.LayerNorm(D)
        for emb in (self.tok, self.pos):
            nn.init.normal_(emb.weight, std=0.02)

    def forward(self, idx):
        x = self.tok(idx) + self.pos(torch.arange(idx.shape[1]))
        for b in self.blocks:
            x = b(x)
        return self.lnf(x) @ self.tok.weight.T  # tied head


def load_corpus(tokens):
    """corpus.txt → LongTensor [N, CTX + 1] of `<theme> <dK> motifs… <eos>`, PAD-filled."""
    ids = {t: i for i, t in enumerate(tokens)}
    seqs = []
    with open(os.path.join(HERE, 'corpus.txt')) as f:
        for line in f:
            s = [ids[w] for w in line.split()] + [EOS]
            assert len(s) <= CTX + 1, f'sequence of {len(s)} tokens exceeds CTX'
            seqs.append(s + [PAD] * (CTX + 1 - len(s)))
    return torch.tensor(seqs)


def _tensors(model):
    sd = model.state_dict()
    out = [('m', sd['tok.weight']), ('m', sd['pos.weight'])]
    for i in range(N_LAYER):
        p = f'blocks.{i}.'
        out += [('v', sd[p + 'ln1.weight']), ('v', sd[p + 'ln1.bias']),
                ('m', sd[p + 'qkv.weight']), ('v', sd[p + 'qkv.bias']),
                ('m', sd[p + 'proj.weight']), ('v', sd[p + 'proj.bias']),
                ('v', sd[p + 'ln2.weight']), ('v', sd[p + 'ln2.bias']),
                ('m', sd[p + 'fc1.weight']), ('v', sd[p + 'fc1.bias']),
                ('m', sd[p + 'fc2.weight']), ('v', sd[p + 'fc2.bias'])]
    out += [('v', sd['lnf.weight']), ('v', sd['lnf.bias'])]
    return out


def _f32(t):
    flat = t.detach().float().flatten().tolist()
    return struct.pack(f'<{len(flat)}f', *flat)


def _header(magic, v):
    return magic + struct.pack('<6I', v, CTX, D, N_LAYER, N_HEAD, D_FF)


def export_f32(model, path):
    v = model.tok.weight.shape[0]
    with open(path, 'wb') as f:
        f.write(_header(b'LGM1', v))
        for _, t in _tensors(model):
            f.write(_f32(t))


def export_int8(model, path):
    v = model.tok.weight.shape[0]
    with open(path, 'wb') as f:
        f.write(_header(b'LGM2', v))
        for kind, t in _tensors(model):
            t = t.detach().float()
            if kind == 'v':
                f.write(_f32(t))
                continue
            scale = t.abs().amax(dim=1).clamp(min=1e-8) / 127.0
            q = torch.round(t / scale[:, None]).clamp(-127, 127).to(torch.int8)
            f.write(_f32(scale))
            f.write(bytes(q.flatten().view(torch.uint8).tolist()))
