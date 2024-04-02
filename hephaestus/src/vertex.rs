use ash::vk::Format;

pub enum AttributeType {
    Vec2,
    Vec3,
    Vec4
}

impl AttributeType {
    pub fn to_format(self) -> Format {
        match self {
            Self::Vec2 => Format::R32G32_SFLOAT,
            Self::Vec3 => Format::R32G32B32_SFLOAT,
            Self::Vec4 => Format::R32G32B32A32_SFLOAT
        }
    }
}

pub struct Info {
    pub stride: usize,
    pub attributes: Vec<(AttributeType, usize)>
}

impl Info {
    pub fn new(stride: usize) -> Self {
        Self { stride, attributes: Vec::new() }
    }

    pub fn attribute(mut self, ty: AttributeType, offset: usize) -> Self {
        self.attributes.push((ty, offset));
        self
    }
}
