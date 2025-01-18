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

On 1080p YUV420 clips, this plugin is about 1.9 times faster than
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
  Time (mean ± σ):     33.475 s ±  0.952 s    [User: 8.109 s, System: 0.141 s]
  Range (min … max):   32.464 s … 34.973 s    10 runs

Benchmark 2: vspipe test.py --end 1999 -o 1 .
  Time (mean ± σ):     49.710 s ±  1.287 s    [User: 18.153 s, System: 6.534 s]
  Range (min … max):   47.882 s … 51.389 s    10 runs

Benchmark 3: vspipe test.py --end 1999 -o 2 .
  Time (mean ± σ):     17.547 s ±  0.071 s    [User: 232.575 s, System: 7.808 s]
  Range (min … max):   17.433 s … 17.640 s    10 runs

Summary
  vspipe test.py --end 1999 -o 2 . ran
    1.91 ± 0.05 times faster than vspipe test.py --end 1999 -o 0 .
    2.83 ± 0.07 times faster than vspipe test.py --end 1999 -o 1 .
```

| Benchmark         | Mean (fps)    | Range (fps)     |
| ----------------- | ------------- | --------------- |
| fftspectrum       | 59.74 ± 1.70  | 57.18 … 58.03   |
| numpy ModifyFrame | 40.23 ± 1.04  | 38.91 … 41.76   |
| fftspectrum_rs    | 113.97 ± 0.46 | 113.37 … 114.72 |

## Build

Rust v1.83.0-nightly and cargo may be used to build the project. Older versions
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
