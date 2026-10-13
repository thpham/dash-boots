#!/usr/bin/env python3
"""GRPO on the level model: rollouts in Rust (the game's own inference and
grammar mask, bot-scored), group-normalised advantages, KL to the SFT model.
Starts from sft.pt; writes rl.pt and — if the int8 export keeps acceptance
within 1 point of f32 — an int8 wasm/assets/levelgen.bin.

  just levelgen-rl              (runs in `nix develop .#training`)
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

import torch

import lgm

STEPS = int(os.environ.get('STEPS', 500))
GROUPS = int(os.environ.get('GROUPS', 16))
SIZE = 16
LR = 1e-4
BETA = float(os.environ.get('BETA', 0.1))
TARGET = os.path.join(lgm.CRATE, 'target', 'release', 'examples')


def rollouts(weights, seed):
    out = subprocess.run([os.path.join(TARGET, 'levelgen_rollout'), '--weights', weights,
                          '--groups', str(GROUPS), '--size', str(SIZE), '--seed', str(seed)],
                         check=True, capture_output=True, text=True).stdout
    return [r for r in map(json.loads, out.splitlines()) if not r.get('failed')]


def batch(rows, v):
    L = max(len(r['tokens']) for r in rows)
    toks = torch.full((len(rows), L), lgm.PAD, dtype=torch.long)
    assert v <= 63, 'masks are u64 bitsets; torch.long holds 63 bits'
    allowed = torch.zeros((len(rows), L - 2, v), dtype=torch.bool)
    bits = torch.tensor([1 << i for i in range(v)], dtype=torch.long)
    for i, r in enumerate(rows):
        toks[i, :len(r['tokens'])] = torch.tensor(r['tokens'])
        m = torch.tensor(r['masks'], dtype=torch.long)
        allowed[i, :len(r['masks'])] = (m[:, None] & bits[None, :]) != 0
        allowed[i, len(r['masks']):, lgm.PAD] = True  # padding only: a harmless target
    return toks, allowed


def masked_logp(model, toks, allowed):
    logits = model(toks[:, :-1])[:, 1:, :]  # positions predicting tokens[2:]
    logits = logits.masked_fill(~allowed, float('-inf'))
    tgt = toks[:, 2:]
    logp = torch.log_softmax(logits, -1).gather(-1, tgt.unsqueeze(-1)).squeeze(-1)
    return logp, tgt != lgm.PAD


def advantages(rows):
    r = torch.tensor([x['reward'] for x in rows])
    g = torch.tensor([x['group'] for x in rows])
    adv = torch.zeros_like(r)
    for k in g.unique():
        sel = g == k
        adv[sel] = (r[sel] - r[sel].mean()) / (r[sel].std(unbiased=False) + 1e-6)
    return adv


def eval_accept(weights):
    out = subprocess.run([os.path.join(TARGET, 'levelgen_eval'), '--weights', weights, '--n', '10'],
                         check=True, capture_output=True, text=True).stdout
    print(out)
    return float(re.search(r'^ACCEPT ([\d.]+)', out, re.M).group(1))


def main():
    torch.manual_seed(0)
    tokens = lgm.vocab()
    v = len(tokens)
    policy, ref = lgm.Model(v), lgm.Model(v)
    sd = torch.load(os.path.join(lgm.HERE, 'sft.pt'), weights_only=True)
    policy.load_state_dict(sd)
    ref.load_state_dict(sd)
    ref.eval()
    for p in ref.parameters():
        p.requires_grad_(False)
    opt = torch.optim.AdamW(policy.parameters(), lr=LR, weight_decay=0.0)
    tmp = tempfile.mkdtemp(prefix='levelgen-grpo-')
    weights = os.path.join(tmp, 'policy.bin')
    t0 = time.time()
    for step in range(STEPS):
        policy.eval()
        lgm.export_f32(policy, weights)
        rows = rollouts(weights, step)
        policy.train()
        toks, allowed = batch(rows, v)
        adv = advantages(rows)
        logp, valid = masked_logp(policy, toks, allowed)
        with torch.no_grad():
            ref_logp, _ = masked_logp(ref, toks, allowed)
        diff = (ref_logp - logp).masked_fill(~valid, 0.0)
        kl = torch.exp(diff) - diff - 1
        pg = -(adv[:, None] * logp).masked_fill(~valid, 0.0)
        loss = (pg.sum() + BETA * kl.sum()) / valid.sum()
        opt.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(policy.parameters(), 1.0)
        opt.step()
        if step % 10 == 0:
            rew = sum(r['reward'] for r in rows) / len(rows)
            fin = sum(r['measured'] is not None for r in rows) / len(rows)
            gap = [r['difficulty'] - r['measured'] for r in rows if r['measured'] is not None]
            print(f'step {step:4d}  reward {rew:+.3f}  finished {fin:.0%}  '
                  f'|target-measured| {sum(gap) / max(1, len(gap)):.2f}  '
                  f'kl {(kl.sum() / valid.sum()).item():.4f}  ({time.time() - t0:.0f}s)', flush=True)
    policy.eval()
    torch.save(policy.state_dict(), os.path.join(lgm.HERE, 'rl.pt'))

    lgm.export_f32(policy, weights)
    q = os.path.join(tmp, 'policy.int8.bin')
    lgm.export_int8(policy, q)
    a32, a8 = eval_accept(weights), eval_accept(q)
    print(f'acceptance f32 {a32:.1f} % · int8 {a8:.1f} %')
    if a8 < a32 - 1.0:
        sys.exit(f'int8 export loses {a32 - a8:.1f} points (> 1): levelgen.bin left unchanged')
    shutil.copyfile(q, lgm.ASSET)
    print(f'wrote {lgm.ASSET} ({os.path.getsize(lgm.ASSET) / 1024:.0f} KiB, int8)')


if __name__ == '__main__':
    sys.exit(main())
