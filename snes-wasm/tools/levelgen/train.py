#!/usr/bin/env python3
"""Supervised pre-training of the level model on the bot-labelled corpus.
Writes tools/levelgen/sft.pt (GRPO start + KL reference) and an int8
wasm/assets/levelgen.bin so the game has a model before RL.

  just levelgen-train          (runs in `nix develop .#training`)
"""

import math
import os
import sys
import time

import torch
import torch.nn.functional as F

import lgm

STEPS = int(os.environ.get('STEPS', 2000))
BATCH, LR = 64, 3e-3


def main():
    torch.manual_seed(0)
    tokens = lgm.vocab()
    data = lgm.load_corpus(tokens)
    perm = torch.randperm(len(data), generator=torch.Generator().manual_seed(0))
    data = data[perm]
    n_val = len(data) // 20
    val, train = data[:n_val], data[n_val:]
    model = lgm.Model(len(tokens))
    print(f'vocab {len(tokens)}, {len(train)} train / {n_val} val, '
          f'{sum(p.numel() for p in model.parameters()) / 1e3:.0f}k params', flush=True)
    opt = torch.optim.AdamW(model.parameters(), lr=LR, weight_decay=0.05)
    sched = lambda s: min(1.0, s / 200) * 0.5 * (1 + math.cos(math.pi * s / STEPS))

    def loss_on(batch):
        n = int((batch != lgm.PAD).sum(1).max())  # trim the padding tail
        x, y = batch[:, :n - 1], batch[:, 1:n].clone()
        y[:, 0] = lgm.PAD  # the difficulty tag is given, not predicted
        logits = model(x)
        return F.cross_entropy(logits.reshape(-1, logits.shape[-1]), y.reshape(-1), ignore_index=lgm.PAD)

    t0 = time.time()
    for step in range(STEPS + 1):
        for g in opt.param_groups:
            g['lr'] = LR * sched(step)
        loss = loss_on(train[torch.randint(len(train), (BATCH,))])
        opt.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
        opt.step()
        if step % 250 == 0:
            with torch.no_grad():
                model.eval()
                vl = loss_on(val).item()
                model.train()
            print(f'step {step:5d}  train {loss.item():.3f}  val {vl:.3f}  ({time.time() - t0:.0f}s)', flush=True)
    model.eval()
    torch.save(model.state_dict(), os.path.join(lgm.HERE, 'sft.pt'))
    lgm.export_int8(model, lgm.ASSET)
    print(f'wrote sft.pt and {lgm.ASSET} ({os.path.getsize(lgm.ASSET) / 1024:.0f} KiB, int8)')


if __name__ == '__main__':
    sys.exit(main())
