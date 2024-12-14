use std::ffi::{c_void, CStr, CString};

use const_str::cstr;
use ndarray::{s, Array, ArrayViewMut};
use rustfft::{num_complex::Complex, FftDirection, FftPlanner};
use vapours::{frame::VapoursVideoFrame, vs_enums::GRAYS};
use vapoursynth4_rs::{
  core::CoreRef,
  declare_plugin,
  frame::{FrameContext, VideoFrame},
  key,
  map::MapRef,
  node::{
    ActivationReason, Dependencies, Filter, FilterDependency, Node, RequestPattern, VideoNode,
  },
  SampleType,
};

#[inline]
fn scale(d: &mut f32, s: &Complex<f64>) {
  *d = (s.norm().ln() / 10.0) as f32;
}

struct FftSpectrumFilter {
  /// Input video node.
  node: VideoNode,
}

impl Filter for FftSpectrumFilter {
  type Error = CString;
  type FrameType = VideoFrame;
  type FilterData = ();

  fn create(
    input: &MapRef,
    output: &mut MapRef,
    _data: Option<Box<Self::FilterData>>,
    mut core: CoreRef<'_>,
  ) -> Result<(), Self::Error> {
    let Ok(node) = input.get_video_node(key!("clip"), 0) else {
      return Err(cstr!("fftspectrum_rs: failed to get clip.").to_owned());
    };

    let n = node.clone();
    let mut vi = n.info().clone();

    if vi.format.bits_per_sample != 32 {
      return Err(
        CString::new(format!(
          "fftspectrum_rs: expected 32-bit input, got {}-bit.",
          vi.format.bits_per_sample
        ))
        .expect("should create CString from String"),
      );
    }

    if vi.format.sample_type != SampleType::Float {
      return Err(cstr!("fftspectrum_rs: expected float input.").to_owned());
    }

    let mut filter = Self { node };

    let deps = [FilterDependency {
      source: filter.node.as_mut_ptr(),
      request_pattern: RequestPattern::StrictSpatial,
    }];

    vi.format = GRAYS;

    core.create_video_filter(
      output,
      cstr!("FFTSpectrum"),
      &vi,
      Box::new(filter),
      Dependencies::new(&deps).unwrap(),
    );

    Ok(())
  }

  fn get_frame(
    &self,
    n: i32,
    activation_reason: ActivationReason,
    _frame_data: *mut *mut c_void,
    mut ctx: FrameContext,
    core: CoreRef<'_>,
  ) -> Result<Option<VideoFrame>, Self::Error> {
    match activation_reason {
      ActivationReason::Initial => {
        ctx.request_frame_filter(n, &self.node);
      }
      ActivationReason::AllFramesReady => {
        let src = self.node.get_frame_filter(n, &mut ctx);
        let width = src.frame_width(0) as usize;
        let height = src.frame_height(0) as usize;

        let mut dst = core.new_video_frame(&GRAYS, width as i32, height as i32, Some(&src));

        // Prepare complex numbers matrix.
        let shape = (height, width);
        let src_complex: Vec<Complex<f64>> = src
          .as_slice::<f32>(0)
          .iter()
          .map(|&x| Complex::new(f64::from(x), 0.0))
          .collect();
        let mut src_arr =
          Array::from_shape_vec(shape, src_complex).expect("should create array from slice");

        // FFT each row.
        let mut planner = FftPlanner::new();
        let fft_width = planner.plan_fft(width, FftDirection::Forward);
        let mut scratch = vec![Complex::default(); fft_width.get_inplace_scratch_len()];
        fft_width.process_with_scratch(src_arr.as_slice_mut().unwrap(), &mut scratch);

        // FFT each column.
        // Parallelizing this loop was slower in benchmarks.
        let fft_height = planner.plan_fft(height, FftDirection::Forward);
        scratch.resize(fft_height.get_inplace_scratch_len(), Complex::default());
        for mut col_buffer in src_arr.columns_mut() {
          // Make column contiguous. This plus the assignment back to the array
          // is a big performance hit but it was still benchmarked as faster
          // than doing full transposes.
          let mut col_owned = col_buffer.to_owned();
          fft_height.process_with_scratch(col_owned.as_slice_mut().unwrap(), &mut scratch);
          col_buffer.assign(&col_owned);
        }

        let half_height = height / 2;
        let half_width = width / 2;

        let top_left = src_arr.slice(s![..half_height, ..half_width]);
        let top_right = src_arr.slice(s![..half_height, half_width..]);
        let bottom_left = src_arr.slice(s![half_height.., ..half_width]);
        let bottom_right = src_arr.slice(s![half_height.., half_width..]);

        // Write to output frame while also shifting low frequencies to the
        // center at the same time. Skipping an intermediate array like this
        // significantly cuts down the processing time.
        let dst_slice = dst.as_mut_slice::<f32>(0);
        let mut dst_arr = ArrayViewMut::from_shape((height, width), dst_slice)
          .expect("should create array from slice");

        // Bottom-right => top-left.
        dst_arr
          .slice_mut(s![..half_height, ..half_width])
          .zip_mut_with(&bottom_right, scale);

        // Bottom-left => top-right.
        dst_arr
          .slice_mut(s![..half_height, half_width..])
          .zip_mut_with(&bottom_left, scale);

        // Top-right => bottom-left.
        dst_arr
          .slice_mut(s![half_height.., ..half_width])
          .zip_mut_with(&top_right, scale);

        // Top-left => bottom-right.
        dst_arr
          .slice_mut(s![half_height.., half_width..])
          .zip_mut_with(&top_left, scale);

        return Ok(Some(dst));
      }
      ActivationReason::Error => {}
    }

    Ok(None)
  }

  const NAME: &'static CStr = cstr!("FFTSpectrum");
  const ARGS: &'static CStr = cstr!("clip:vnode;");
  const RETURN_TYPE: &'static CStr = cstr!("clip:vnode;");
}

declare_plugin!(
  "sgt.fftspectrum_rs",
  "fftspectrum_rs",
  "FFT frequency spectrum.",
  (0, 0),
  VAPOURSYNTH_API_VERSION,
  0,
  (FftSpectrumFilter, None)
);
