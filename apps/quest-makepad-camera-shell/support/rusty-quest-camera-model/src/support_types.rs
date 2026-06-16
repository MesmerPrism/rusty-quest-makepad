use core::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn splat(value: f32) -> Self {
        Self::new(value, value)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from(value: [f32; 2]) -> Self {
        Self::new(value[0], value[1])
    }
}

impl From<Vec2> for [f32; 2] {
    fn from(value: Vec2) -> Self {
        [value.x, value.y]
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    pub const RIGHT: Self = Self::new(1.0, 0.0, 0.0);
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);
    pub const FORWARD_NEG_Z: Self = Self::new(0.0, 0.0, -1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn splat(value: f32) -> Self {
        Self::new(value, value, value)
    }

    pub fn dot(self, other: Self) -> f32 {
        (self.x * other.x) + (self.y * other.y) + (self.z * other.z)
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            (self.y * other.z) - (self.z * other.y),
            (self.z * other.x) - (self.x * other.z),
            (self.x * other.y) - (self.y * other.x),
        )
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalized_or(self, fallback: Self) -> Self {
        let len_sq = self.length_squared();
        if len_sq <= 1.0e-12 || !len_sq.is_finite() {
            fallback
        } else {
            self / len_sq.sqrt()
        }
    }

    pub fn clamped_length(self, max_length: f32) -> Self {
        let max_length = max_length.max(0.0);
        if max_length <= 1.0e-6 {
            return Self::ZERO;
        }

        let len_sq = self.length_squared();
        let max_sq = max_length * max_length;
        if len_sq > max_sq {
            self.normalized_or(Self::ZERO) * max_length
        } else {
            self
        }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self::new(
            self.x.clamp(min.x, max.x),
            self.y.clamp(min.y, max.y),
            self.z.clamp(min.z, max.z),
        )
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(value: [f32; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}

impl From<Vec3> for [f32; 3] {
    fn from(value: Vec3) -> Self {
        [value.x, value.y, value.z]
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn from_axis_angle(axis: Vec3, radians: f32) -> Self {
        let axis = axis.normalized_or(Vec3::UP);
        let half = radians * 0.5;
        let (sin_half, cos_half) = half.sin_cos();
        Self::new(
            axis.x * sin_half,
            axis.y * sin_half,
            axis.z * sin_half,
            cos_half,
        )
        .normalized_or(Self::IDENTITY)
    }

    pub fn length_squared(self) -> f32 {
        (self.x * self.x) + (self.y * self.y) + (self.z * self.z) + (self.w * self.w)
    }

    pub fn normalized_or(self, fallback: Self) -> Self {
        let len_sq = self.length_squared();
        if len_sq <= 1.0e-12 || !len_sq.is_finite() {
            fallback
        } else {
            self.scale(1.0 / len_sq.sqrt())
        }
    }

    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    pub fn inverse(self) -> Self {
        let len_sq = self.length_squared();
        if len_sq <= 1.0e-12 || !len_sq.is_finite() {
            Self::IDENTITY
        } else {
            self.conjugate().scale(1.0 / len_sq)
        }
    }

    pub fn rotate_vec3(self, value: Vec3) -> Vec3 {
        let q = self.normalized_or(Self::IDENTITY);
        let qv = Vec3::new(q.x, q.y, q.z);
        let t = qv.cross(value) * 2.0;
        value + (t * q.w) + qv.cross(t)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }

    fn scale(self, value: f32) -> Self {
        Self::new(
            self.x * value,
            self.y * value,
            self.z * value,
            self.w * value,
        )
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul for Quat {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            (self.w * rhs.x) + (self.x * rhs.w) + (self.y * rhs.z) - (self.z * rhs.y),
            (self.w * rhs.y) - (self.x * rhs.z) + (self.y * rhs.w) + (self.z * rhs.x),
            (self.w * rhs.z) + (self.x * rhs.y) - (self.y * rhs.x) + (self.z * rhs.w),
            (self.w * rhs.w) - (self.x * rhs.x) - (self.y * rhs.y) - (self.z * rhs.z),
        )
    }
}

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    pub orientation: Quat,
}

impl Pose {
    pub const IDENTITY: Self = Self::new(Vec3::ZERO, Quat::IDENTITY);

    pub const fn new(position: Vec3, orientation: Quat) -> Self {
        Self {
            position,
            orientation,
        }
    }

    pub fn transform_point(self, point: Vec3) -> Vec3 {
        self.position + self.orientation.rotate_vec3(point)
    }

    pub fn inverse_transform_point(self, point: Vec3) -> Vec3 {
        self.orientation
            .inverse()
            .rotate_vec3(point - self.position)
    }

    pub fn is_finite(self) -> bool {
        self.position.is_finite() && self.orientation.is_finite()
    }
}

impl Default for Pose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

impl ImageSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn is_non_empty(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eye {
    Mono,
    Left,
    Right,
}

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect2 {
    pub origin: Vec2,
    pub size: Vec2,
}

impl Rect2 {
    pub const UNIT: Self = Self::new(Vec2::ZERO, Vec2::ONE);

    pub const fn new(origin: Vec2, size: Vec2) -> Self {
        Self { origin, size }
    }

    pub fn max(self) -> Vec2 {
        self.origin + self.size
    }

    pub fn center(self) -> Vec2 {
        self.origin + (self.size * 0.5)
    }

    pub fn aspect(self) -> Option<f32> {
        if self.size.x > 0.0 && self.size.y > 0.0 && self.size.is_finite() {
            Some(self.size.x / self.size.y)
        } else {
            None
        }
    }

    pub fn is_valid(self) -> bool {
        self.origin.is_finite() && self.size.is_finite() && self.size.x >= 0.0 && self.size.y >= 0.0
    }

    pub fn inset(self, amount: f32) -> Option<Self> {
        if !self.is_valid() || !amount.is_finite() || amount < 0.0 {
            return None;
        }

        let max_inset = (self.size.x.min(self.size.y) * 0.5).max(0.0);
        let inset = amount.min(max_inset);
        Some(Self::new(
            Vec2::new(self.origin.x + inset, self.origin.y + inset),
            Vec2::new(
                (self.size.x - (inset * 2.0)).max(0.0),
                (self.size.y - (inset * 2.0)).max(0.0),
            ),
        ))
    }

    pub fn aspect_fit(self, content_aspect: f32) -> Option<Self> {
        self.aspect_layout(content_aspect, StereoLayerContentMode::Fit)
    }

    pub fn aspect_fill(self, content_aspect: f32) -> Option<Self> {
        self.aspect_layout(content_aspect, StereoLayerContentMode::Fill)
    }

    fn aspect_layout(self, content_aspect: f32, mode: StereoLayerContentMode) -> Option<Self> {
        if !self.is_valid()
            || self.size.x <= 0.0
            || self.size.y <= 0.0
            || !content_aspect.is_finite()
            || content_aspect <= 0.0
        {
            return None;
        }

        if matches!(mode, StereoLayerContentMode::Stretch) {
            return Some(self);
        }

        let container_aspect = self.size.x / self.size.y;
        let fit_by_width = match mode {
            StereoLayerContentMode::Fit => container_aspect <= content_aspect,
            StereoLayerContentMode::Fill => container_aspect > content_aspect,
            StereoLayerContentMode::Stretch => unreachable!(),
        };

        let size = if fit_by_width {
            Vec2::new(self.size.x, self.size.x / content_aspect)
        } else {
            Vec2::new(self.size.y * content_aspect, self.size.y)
        };
        let origin = Vec2::new(
            self.origin.x + ((self.size.x - size.x) * 0.5),
            self.origin.y + ((self.size.y - size.y) * 0.5),
        );

        Some(Self::new(origin, size))
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn is_finite(self) -> bool {
        self.r.is_finite() && self.g.is_finite() && self.b.is_finite() && self.a.is_finite()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraIntrinsics {
    pub focal_length_px: Vec2,
    pub principal_point_px: Vec2,
    pub skew_px: f32,
    pub image_size: ImageSize,
}

impl CameraIntrinsics {
    pub const fn new(
        focal_length_px: Vec2,
        principal_point_px: Vec2,
        image_size: ImageSize,
    ) -> Self {
        Self {
            focal_length_px,
            principal_point_px,
            skew_px: 0.0,
            image_size,
        }
    }

    pub const fn with_skew_px(mut self, skew_px: f32) -> Self {
        self.skew_px = skew_px;
        self
    }

    pub fn is_valid(self) -> bool {
        self.image_size.is_non_empty()
            && self.focal_length_px.is_finite()
            && self.principal_point_px.is_finite()
            && self.skew_px.is_finite()
            && self.focal_length_px.x > 0.0
            && self.focal_length_px.y > 0.0
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraExtrinsics {
    pub world_from_camera: Pose,
}

impl CameraExtrinsics {
    pub const fn new(world_from_camera: Pose) -> Self {
        Self { world_from_camera }
    }

    pub fn is_valid(self) -> bool {
        self.world_from_camera.is_finite()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraImageRotation {
    #[default]
    Rotate0,
    Rotate90,
    Rotate180,
    Rotate270,
}

impl CameraImageRotation {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Rotate0 => "rotate0",
            Self::Rotate90 => "rotate90",
            Self::Rotate180 => "rotate180",
            Self::Rotate270 => "rotate270",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "0" | "rotate0" | "none" => Some(Self::Rotate0),
            "90" | "rotate90" => Some(Self::Rotate90),
            "180" | "rotate180" => Some(Self::Rotate180),
            "270" | "rotate270" => Some(Self::Rotate270),
            _ => None,
        }
    }

    pub const fn shader_bits(self) -> u32 {
        match self {
            Self::Rotate0 => 0,
            Self::Rotate90 => 1,
            Self::Rotate180 => 2,
            Self::Rotate270 => 3,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraTextureTransform {
    pub rotation: CameraImageRotation,
    pub flip_x: bool,
    pub flip_y: bool,
    pub mirror: bool,
    pub source_label: String,
    pub reason: String,
}

impl CameraTextureTransform {
    pub fn new(source_label: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            rotation: CameraImageRotation::Rotate0,
            flip_x: false,
            flip_y: false,
            mirror: false,
            source_label: source_label.into(),
            reason: reason.into(),
        }
    }

    pub const fn with_rotation(mut self, rotation: CameraImageRotation) -> Self {
        self.rotation = rotation;
        self
    }

    pub const fn with_flip_x(mut self, flip_x: bool) -> Self {
        self.flip_x = flip_x;
        self
    }

    pub const fn with_flip_y(mut self, flip_y: bool) -> Self {
        self.flip_y = flip_y;
        self
    }

    pub const fn with_mirror(mut self, mirror: bool) -> Self {
        self.mirror = mirror;
        self
    }

    pub fn is_explicit_visual_check(&self) -> bool {
        !self.source_label.trim().is_empty()
            && !self.reason.trim().is_empty()
            && self.source_label != "default"
            && self.reason != "unspecified"
    }

    pub fn shader_flags(&self) -> u32 {
        self.rotation.shader_bits()
            | ((self.flip_x as u32) << 2)
            | ((self.flip_y as u32) << 3)
            | ((self.mirror as u32) << 4)
    }

    pub fn label(&self) -> String {
        let mut parts = vec![self.rotation.stable_id().to_string()];
        if self.flip_x {
            parts.push("flipX".to_string());
        }
        if self.flip_y {
            parts.push("flipY".to_string());
        }
        if self.mirror {
            parts.push("mirror".to_string());
        }
        parts.join("+")
    }

    pub fn apply_uv(&self, uv: Vec2) -> Vec2 {
        let mut result = match self.rotation {
            CameraImageRotation::Rotate0 => uv,
            CameraImageRotation::Rotate90 => Vec2::new(uv.y, 1.0 - uv.x),
            CameraImageRotation::Rotate180 => Vec2::new(1.0 - uv.x, 1.0 - uv.y),
            CameraImageRotation::Rotate270 => Vec2::new(1.0 - uv.y, uv.x),
        };
        if self.flip_x || self.mirror {
            result.x = 1.0 - result.x;
        }
        if self.flip_y {
            result.y = 1.0 - result.y;
        }
        result
    }
}

impl Default for CameraTextureTransform {
    fn default() -> Self {
        Self::new("default", "unspecified")
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StereoMediaLayout {
    #[default]
    Mono,
    SideBySide {
        left_first: bool,
    },
    TopBottom {
        left_first: bool,
    },
    Separate,
}

impl StereoMediaLayout {
    pub const SIDE_BY_SIDE_LEFT_FIRST: Self = Self::SideBySide { left_first: true };
    pub const TOP_BOTTOM_LEFT_FIRST: Self = Self::TopBottom { left_first: true };

    pub fn eye_uv_rect(self, eye: Eye) -> Rect2 {
        match self {
            Self::Mono | Self::Separate => Rect2::UNIT,
            Self::SideBySide { left_first } => {
                let left_is_low_half = left_first;
                let use_low_half = matches!(eye, Eye::Left) == left_is_low_half;
                let x = if use_low_half { 0.0 } else { 0.5 };
                Rect2::new(Vec2::new(x, 0.0), Vec2::new(0.5, 1.0))
            }
            Self::TopBottom { left_first } => {
                let left_is_low_half = left_first;
                let use_low_half = matches!(eye, Eye::Left) == left_is_low_half;
                let y = if use_low_half { 0.0 } else { 0.5 };
                Rect2::new(Vec2::new(0.0, y), Vec2::new(1.0, 0.5))
            }
        }
    }

    pub fn eye_source_size(self, source_size: ImageSize) -> Option<Vec2> {
        if !source_size.is_non_empty() {
            return None;
        }

        let width = source_size.width as f32;
        let height = source_size.height as f32;
        let size = match self {
            Self::Mono | Self::Separate => Vec2::new(width, height),
            Self::SideBySide { .. } => Vec2::new(width * 0.5, height),
            Self::TopBottom { .. } => Vec2::new(width, height * 0.5),
        };

        if size.x > 0.0 && size.y > 0.0 {
            Some(size)
        } else {
            None
        }
    }

    pub fn eye_aspect(self, source_size: ImageSize) -> Option<f32> {
        let size = self.eye_source_size(source_size)?;
        Some(size.x / size.y)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum StereoLayerContentMode {
    #[default]
    Fit,
    Fill,
    Stretch,
}

pub const SOURCE_SAMPLING_CONTRACT_SCHEMA: &str = "rusty.optics.source-sampling-contract.v1";

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StereoSourceEyeMapping {
    #[default]
    DisplayLeftFromLeftSource,
    DisplayLeftFromRightSource,
}

impl StereoSourceEyeMapping {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "left-right"
            | "display-left-from-left"
            | "display-left-from-left-source"
            | "displayleftfromleft"
            | "natural"
            | "camera50-left" => Some(Self::DisplayLeftFromLeftSource),
            "right-left"
            | "display-left-from-right"
            | "display-left-from-right-source"
            | "displayleftfromright"
            | "swapped"
            | "swap"
            | "camera51-left" => Some(Self::DisplayLeftFromRightSource),
            _ => None,
        }
    }

    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::DisplayLeftFromLeftSource => "display-left-from-left-source",
            Self::DisplayLeftFromRightSource => "display-left-from-right-source",
        }
    }

    pub const fn swaps_display_source_eyes(self) -> bool {
        matches!(self, Self::DisplayLeftFromRightSource)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceSamplingTransformStage {
    #[default]
    None,
    PostHomographyPreTextureSample,
    PostHomographyPreOesSample,
    PostHomographyPreYuvSample,
    PostHomographyPreSourceVisibleRectThenTextureSample,
    Other,
}

impl SourceSamplingTransformStage {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PostHomographyPreTextureSample => "post-homography-pre-texture-sample",
            Self::PostHomographyPreOesSample => "post-homography-pre-oes-sample",
            Self::PostHomographyPreYuvSample => "post-homography-pre-yuv-sample",
            Self::PostHomographyPreSourceVisibleRectThenTextureSample => {
                "post-homography-pre-source-visible-rect-then-texture-sample"
            }
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" | "off" => Some(Self::None),
            "post-homography-pre-texture-sample" | "post_homography_pre_texture_sample" => {
                Some(Self::PostHomographyPreTextureSample)
            }
            "post-homography-pre-oes-sample" | "post_homography_pre_oes_sample" => {
                Some(Self::PostHomographyPreOesSample)
            }
            "post-homography-pre-yuv-sample" | "post_homography_pre_yuv_sample" => {
                Some(Self::PostHomographyPreYuvSample)
            }
            "post-homography-pre-source-visible-rect-then-texture-sample"
            | "post_homography_pre_source_visible_rect_then_texture_sample" => {
                Some(Self::PostHomographyPreSourceVisibleRectThenTextureSample)
            }
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceSamplerYAxis {
    #[default]
    RendererDefined,
    SurfaceTextureTransformDefined,
    ContentTopLeftYDown,
    MakepadSamplerOriginConvention,
    Other,
}

impl SourceSamplerYAxis {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::RendererDefined => "renderer-defined",
            Self::SurfaceTextureTransformDefined => "surface-texture-transform-defined",
            Self::ContentTopLeftYDown => "content-top-left-y-down",
            Self::MakepadSamplerOriginConvention => "makepad-sampler-origin-convention",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "renderer-defined" | "renderer_defined" => Some(Self::RendererDefined),
            "surface-texture-transform-defined" | "surface_texture_transform_defined" => {
                Some(Self::SurfaceTextureTransformDefined)
            }
            "content-top-left-y-down" | "content_top_left_y_down" => {
                Some(Self::ContentTopLeftYDown)
            }
            "makepad-sampler-origin-convention" | "makepad_sampler_origin_convention" => {
                Some(Self::MakepadSamplerOriginConvention)
            }
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceUvRect {
    pub origin_uv: Vec2,
    pub size_uv: Vec2,
}

impl SourceUvRect {
    pub const FULL: Self = Self::new(Vec2::ZERO, Vec2::ONE);

    pub const fn new(origin_uv: Vec2, size_uv: Vec2) -> Self {
        Self { origin_uv, size_uv }
    }

    pub fn is_valid(self) -> bool {
        const EPSILON: f32 = 1.0e-5;
        self.origin_uv.is_finite()
            && self.size_uv.is_finite()
            && self.origin_uv.x >= -EPSILON
            && self.origin_uv.y >= -EPSILON
            && self.size_uv.x > 0.0
            && self.size_uv.y > 0.0
            && self.origin_uv.x + self.size_uv.x <= 1.0 + EPSILON
            && self.origin_uv.y + self.size_uv.y <= 1.0 + EPSILON
    }
}

impl Default for SourceUvRect {
    fn default() -> Self {
        Self::FULL
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SourceSamplingContract {
    pub schema_version: String,
    pub backend: String,
    pub mode: String,
    pub source_eye_mapping: StereoSourceEyeMapping,
    pub content_uv_rect: SourceUvRect,
    pub source_visible_uv_rect: SourceUvRect,
    pub transform_stage: SourceSamplingTransformStage,
    pub transform_label: String,
    pub transform_owner: String,
    pub transform_applied: bool,
    pub output_uv_label: String,
    pub sampler_uv_origin: String,
    pub sampler_y_axis: SourceSamplerYAxis,
    pub texture_transform_stage: SourceSamplingTransformStage,
    pub texture_transform_owner: String,
}

impl SourceSamplingContract {
    pub fn new(
        backend: impl Into<String>,
        mode: impl Into<String>,
        source_eye_mapping: StereoSourceEyeMapping,
        transform_stage: SourceSamplingTransformStage,
    ) -> Self {
        Self {
            schema_version: SOURCE_SAMPLING_CONTRACT_SCHEMA.to_string(),
            backend: backend.into(),
            mode: mode.into(),
            source_eye_mapping,
            content_uv_rect: SourceUvRect::FULL,
            source_visible_uv_rect: SourceUvRect::FULL,
            transform_stage,
            transform_label: "identity".to_string(),
            transform_owner: "renderer".to_string(),
            transform_applied: false,
            output_uv_label: "sampler-uv".to_string(),
            sampler_uv_origin: "renderer-defined".to_string(),
            sampler_y_axis: SourceSamplerYAxis::RendererDefined,
            texture_transform_stage: SourceSamplingTransformStage::PostHomographyPreTextureSample,
            texture_transform_owner: "renderer".to_string(),
        }
    }

    pub const fn with_content_uv_rect(mut self, rect: SourceUvRect) -> Self {
        self.content_uv_rect = rect;
        self
    }

    pub const fn with_source_visible_uv_rect(mut self, rect: SourceUvRect) -> Self {
        self.source_visible_uv_rect = rect;
        self
    }

    pub fn with_transform(
        mut self,
        label: impl Into<String>,
        owner: impl Into<String>,
        applied: bool,
    ) -> Self {
        self.transform_label = label.into();
        self.transform_owner = owner.into();
        self.transform_applied = applied;
        self
    }

    pub fn with_sampler(
        mut self,
        output_uv_label: impl Into<String>,
        sampler_uv_origin: impl Into<String>,
        sampler_y_axis: SourceSamplerYAxis,
    ) -> Self {
        self.output_uv_label = output_uv_label.into();
        self.sampler_uv_origin = sampler_uv_origin.into();
        self.sampler_y_axis = sampler_y_axis;
        self
    }

    pub fn with_texture_transform(
        mut self,
        stage: SourceSamplingTransformStage,
        owner: impl Into<String>,
    ) -> Self {
        self.texture_transform_stage = stage;
        self.texture_transform_owner = owner.into();
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema_version == SOURCE_SAMPLING_CONTRACT_SCHEMA
            && !self.backend.trim().is_empty()
            && !self.mode.trim().is_empty()
            && self.content_uv_rect.is_valid()
            && self.source_visible_uv_rect.is_valid()
            && !self.transform_label.trim().is_empty()
            && !self.transform_owner.trim().is_empty()
            && !self.output_uv_label.trim().is_empty()
            && !self.sampler_uv_origin.trim().is_empty()
            && !self.texture_transform_owner.trim().is_empty()
    }
}
