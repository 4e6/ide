use crate::prelude::*;

use crate::component::visualization::{DataRenderer,DataType,Data,DataError};

use ensogl::display::{DomSymbol, DomScene};
use std::rc::Rc;
use ensogl::display;
use ensogl::system::web;
use web::StyleSetter;
use ensogl::gui::component;
use ensogl::display::scene::ShapeRegistry;
use ensogl::display::scene::Scene;
use ensogl::display::layout::alignment;


pub struct HtmlBubbleChart {
    pub content: Rc<DomSymbol>
}

impl DataRenderer for HtmlBubbleChart {

    fn set_data(&self, data:Data) ->  Result<Data,DataError>{
        let mut svg_inner = String::new();

        let data_inner: Rc<Vec<Vector2<f32>>> = data.as_binary()?;
        for pos in data_inner.iter() {
            svg_inner.push_str(
                &format!(
                    r#"<circle style="fill: #69b3a2" stroke="black" cx={} cy={} r=10.0></circle>"#,
                    pos.x, pos.y
                )
            )
        }
        self.content.dom().set_inner_html(
            &format!(r#"
<svg>
{}
</svg>
"#, svg_inner));
    Ok(data)
    }

    fn valid_input_types(&self) -> Vec<DataType> {
        unimplemented!()
    }

    fn set_size(&self, size:Vector2<f32>) {
        self.content.set_size(size);
    }
}

impl HtmlBubbleChart {
    pub fn new() -> Self {
        let div = web::create_div();
        div.set_style_or_panic("width","100px");
        div.set_style_or_panic("height","100px");

        let content = web::create_element("div");
        content.set_inner_html(
            r#"<svg>
<circle style="fill: #69b3a2" stroke="black" cx=50 cy=50 r=20></circle>
</svg>
"#);
        content.set_attribute("width","100%").unwrap();
        content.set_attribute("height","100%").unwrap();

        div.append_child(&content).unwrap();

        let r          = 102_u8;
        let g          = 153_u8;
        let b          = 194_u8;
        let color      = iformat!("rgb({r},{g},{b})");
        div.set_style_or_panic("background-color",color);

        let symbol = DomSymbol::new(&div);
        symbol.dom().set_attribute("id","vis").unwrap();
        symbol.dom().style().set_property("overflow","hidden").unwrap();

        let content = Rc::new(symbol);
        content.set_size(Vector2::new(100.0, 100.0));
        content.set_position(Vector3::new(0.0, 0.0, 0.0));

        HtmlBubbleChart { content }
     }

    pub fn set_dom_layer(&self, scene:&DomScene) {
        scene.manage(&self.content);
    }
}

impl Default for HtmlBubbleChart {
    fn default() -> Self {
        Self::new()
    }
}

impl display::Object for HtmlBubbleChart {
    fn display_object(&self) -> &display::object::Instance {
        &self.content.display_object()
    }
}



/// Canvas node shape definition.
pub mod shape {
    use super::*;
    use ensogl::display::shape::*;
    use ensogl::display::scene::Scene;
    use ensogl::data::color::*;
    use ensogl::display::Sprite;
    use ensogl::display::Buffer;
    use ensogl::display::Attribute;

    ensogl::define_shape_system! {
        (position:Vector2<f32>, radius:f32) {

            let node = Circle(radius);
            let node = node.translate(("input_position.x","input_position.y"));
            let node = node.fill(Srgb::new(0.17,0.46,0.15));

            node.into()
        }
    }
}

/// Shape view for Node.
#[derive(Debug,Clone,Copy)]
pub struct BubbleView {}
impl component::ShapeViewDefinition for BubbleView {
    type Shape = shape::Shape;
    fn new(shape:&Self::Shape, _scene:&Scene, shape_registry:&ShapeRegistry) -> Self {
        shape.sprite.size().set(Vector2::new(100.0,100.0));
        let shape_system = shape_registry.shape_system(PhantomData::<shape::Shape>);
        shape_system.shape_system.set_alignment(alignment::HorizontalAlignment::Center,
                                                alignment::VerticalAlignment::Center);
        shape.radius.set(10.0);
        Self {}
    }
}
pub struct WebglBubbleChart {
    pub node: display::object::Instance,
    views: RefCell<Vec<component::ShapeView<BubbleView>>>,
    logger        : Logger,
}

impl WebglBubbleChart {
    pub fn new() -> Self {
        let logger      = Logger::new("bubble");
        let node  = display::object::Instance::new(&logger);
        node.set_position(Vector3::new(-50.0, -50.0, 0.0));
        let views = RefCell::new(vec![]);

        WebglBubbleChart { node,views,logger }
    }
}

impl DataRenderer for WebglBubbleChart {
    fn set_data(&self, data:Data) -> Result<Data,DataError> {
        let data_inner: Rc<Vec<Vector2<f32>>> = data.as_binary()?;

        let mut views = self.views.borrow_mut();
        views.clear();
        views.extend(data_inner.iter().map(|item| {
            let view =  component::ShapeView::new(&self.logger);
            view.display_object.set_position(Vector3::new(item.x, item.y, 0.0));
            view.display_object.set_parent(&self.node);
            view
        }));
        Ok(data)
    }

    fn valid_input_types(&self) -> Vec<DataType> {
        unimplemented!()
    }

    fn set_size(&self, _size:Vector2<f32>) {
        // unimplemented!()
    }
}

impl display::Object  for WebglBubbleChart {
    fn display_object(&self) -> &display::object::Instance {
        &self.node.display_object()
    }
}

impl Default for WebglBubbleChart {
    fn default() -> Self {
        Self::new()
    }
}