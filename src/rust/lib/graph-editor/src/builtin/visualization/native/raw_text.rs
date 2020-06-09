//! Example visualisation showing the provided data as text.

use crate::prelude::*;

use crate::component::visualization::*;
use crate::component::visualization;
use crate::frp;

use ensogl::display::DomSymbol;
use ensogl::display::scene::Scene;
use ensogl::display;
use ensogl::system::web;
use ensogl::system::web::StyleSetter;



// ===============
// === RawText ===
// ===============

/// Sample visualization that renders the given data as text. Useful for debugging and testing.
#[derive(Debug,Shrinkwrap)]
#[allow(missing_docs)]
pub struct RawText {
    #[shrinkwrap(main_field)]
    model : RawTextModel,
    frp   : visualization::instance::Frp,
}

impl RawText {
    /// Definition of this visualization.
    pub fn definition() -> Definition {
        Definition::new(
            Signature::new_for_any_type(Path::builtin("Raw Text Visualization (native)")),
            |scene| { Ok(Self::new(scene).into()) }
        )
    }

    /// Constructor.
    pub fn new(scene:&Scene) -> Self {
        let frp   = visualization::instance::Frp::new();
        let model = RawTextModel::new(scene);
        Self {model,frp} . init()
    }

    fn init(self) -> Self {
        let network = &self.frp.network;
        let model   = &self.model;
        frp::extend! { network
            eval self.frp.set_size  ((size) model.set_size(*size));
            eval self.frp.send_data ((data) model.receive_data(data).unwrap()); // FIXME : on error emit it via FRP
        }
        self
    }
}

#[derive(Clone,CloneRef,Debug)]
#[allow(missing_docs)]
pub struct RawTextModel {
    logger : Logger,
    dom    : DomSymbol,
    size   : Rc<Cell<V2>>,
}

impl RawTextModel {
    /// Constructor.
    fn new(scene:&Scene) -> Self {
        let logger  = Logger::new("RawText");
        let div     = web::create_div();
        let dom     = DomSymbol::new(&div);
        let size    = Rc::new(Cell::new(V2(200.0,200.0)));

        dom.dom().set_style_or_warn("white-space"   ,"pre"                  ,&logger);
        dom.dom().set_style_or_warn("overflow-y"    ,"auto"                 ,&logger);
        dom.dom().set_style_or_warn("overflow-x"    ,"auto"                 ,&logger);
        dom.dom().set_style_or_warn("font-family"   ,"dejavuSansMono"       ,&logger);
        dom.dom().set_style_or_warn("font-size"     ,"11px"                 ,&logger);
        dom.dom().set_style_or_warn("margin-left"   ,"12px"                 ,&logger);
        dom.dom().set_style_or_warn("color"         ,"rgba(255,255,255,0.7)",&logger);
        dom.dom().set_style_or_warn("pointer-events","auto"                 ,&logger);

        scene.dom.layers.main.manage(&dom);
        RawTextModel{dom,logger,size}.init()
    }

    fn init(self) -> Self {
        self.reload_style();
        self
    }

    fn set_size(&self, size:V2) {
        self.size.set(size);
        self.reload_style();
    }

    fn receive_data(&self, data:&Data) -> Result<(),DataError> {
        let data_inner = match data {
            Data::Json {content} => content,
            _ => todo!() // FIXME
        };
        let data_str = serde_json::to_string_pretty(&**data_inner);
        let data_str = data_str.unwrap_or_else(|e| format!("<Cannot render data: {}>", e));
        let data_str = format!("\n{}",data_str);
        self.dom.dom().set_inner_text(&data_str);
        Ok(())
    }

    fn reload_style(&self) {
        let size = self.size.get();
        self.dom.set_size(Vector2::new(size.x,size.y));
    }
}

impl From<RawText> for Instance {
    fn from(t:RawText) -> Self {
        Self::new(&t,&t.frp)
    }
}

impl display::Object for RawText {
    fn display_object(&self) -> &display::object::Instance {
        &self.dom.display_object()
    }
}