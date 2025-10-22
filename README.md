# vapoursynth-fftspectrum-rs

A faster FFT spectrum [VapourSynth][] plugin.

![example](.github/assets/example.png)

## Install

Via [vsrepo][]:

```
vsrepo install fftspectrum_rs
```

Manually: download a release from the [Releases][] page and unzip
`fftspectrum_rs.dll` (Windows), `libfftspectrum_rs.so` (Linux), or
`libfftspectrum_rs.dylib` (macOS) into a [plugins directory][plugin-autoloading].
There are separate artifacts for Raptor Lake (`*-raptorlake.zip`) and
AMD Zen 4 (`*-znver4.zip`) CPUs which may or may not have better performance
than the plain x86_64 build.

## API

```python
fftspectrum_rs.FFTSpectrum(clip: vs.VideoNode) -> vs.VideoNode
```

- `clip` — Input video node. Must have 32-bit float format.

Only the first plane of `clip` will be processed. The output is a GRAYS video
node.

## Benchmark

On 1080p clips, this plugin is about 2.2 times faster than
`fftspectrum.FFTSpectrum()`. This is mostly accomplished by minimizing the
number of allocations, copies, and transposes done.

```python
import numpy as np
from cv2 import DFT_COMPLEX_OUTPUT, dft
from vssource import BestSource
from vstools import core, depth, get_y, set_output, vs

core.set_affinity(range(0, 32, 2), 22000)

src = BestSource.source("/path/to/1080_clip.mkv", bits=0)
src_8 = depth(src, 8)
src_32 = depth(src, 32)

# fftspectrum
set_output(src_8.fftspectrum.FFTSpectrum(), "fftspectrum")

# numpy + ModifyFrame
def _to_polar(f: vs.VideoFrame, n: int) -> vs.VideoFrame:
    src = np.asarray(f[0])
    dft_shift = np.fft.fftshift(dft(src, flags=DFT_COMPLEX_OUTPUT))
    mag = np.sqrt(np.power(dft_shift[:, :, 1], 2) + np.power(dft_shift[:, :, 0], 2))
    dst = f.copy()
    np.copyto(np.asarray(dst[0]), np.log(mag) / 10)
    return dst

y = get_y(src_32)
set_output(y.std.ModifyFrame(y, _to_polar), "numpy ModifyFrame")

# fftspectrum_rs
set_output(src_32.fftspectrum_rs.FFTSpectrum(), "fftspectrum_rs")
```

```bash
$ hyperfine --warmup 1 'vspipe test.py --end 1999 -o {output_node} .' -P output_node 0 2
Benchmark 1: vspipe test.py --end 1999 -o 0 .
  Time (mean ± σ):     31.878 s ±  0.853 s    [User: 44.497 s, System: 1.592 s]
  Range (min … max):   30.672 s … 33.681 s    10 runs

Benchmark 2: vspipe test.py --end 1999 -o 1 .
  Time (mean ± σ):     62.379 s ±  1.839 s    [User: 59.013 s, System: 22.801 s]
  Range (min … max):   59.342 s … 64.992 s    10 runs

Benchmark 3: vspipe test.py --end 1999 -o 2 .
  Time (mean ± σ):     13.909 s ±  0.146 s    [User: 178.334 s, System: 18.904 s]
  Range (min … max):   13.669 s … 14.166 s    10 runs

Summary
  vspipe test.py --end 1999 -o 2 . ran
    2.29 ± 0.07 times faster than vspipe test.py --end 1999 -o 0 .
    4.48 ± 0.14 times faster than vspipe test.py --end 1999 -o 1 .
```

| Benchmark         | Mean (fps)    | Range (fps)     |
| ----------------- | ------------- | --------------- |
| fftspectrum       | 62.74 ± 1.68  | 59.38 … 65.21   |
| numpy ModifyFrame | 32.06 ± 0.95  | 30.77 … 33.70   |
| fftspectrum_rs    | 143.79 ± 1.51 | 141.18 … 146.32 |

## Build

Rust v1.91.0-nightly and cargo may be used to build the project. Older versions
will likely work fine but they aren't explicitly supported.

```bash
$ git clone https://github.com/sgt0/vapoursynth-fftspectrum-rs.git
$ cd vapoursynth-fftspectrum-rs

# Debug build.
$ cargo build

# Release (optimized) build.
$ cargo build --release

# Release build optimized for the host CPU.
$ RUSTFLAGS="-C target-cpu=native" cargo build --release
```

[VapourSynth]: https://www.vapoursynth.com
[vsrepo]: https://github.com/vapoursynth/vsrepo
[Releases]: https://github.com/sgt0/vapoursynth-fftspectrum-rs/releases
[plugin-autoloading]: https://www.vapoursynth.com/doc/installation.html#plugin-autoloading
