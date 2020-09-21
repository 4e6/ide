//! Definition of the QuickActionBar component.

use crate::prelude::*;

use crate::component::node;
use crate::component::visualization::container::visualization_chooser;
use crate::component::visualization;


use enso_frp as frp;
use enso_frp;
use ensogl::data::color;
use ensogl::display::shape::*;
use ensogl::display::shape::text::glyph::system::GlyphSystem;
use ensogl::display::traits::*;
use ensogl::display;
use ensogl::gui::component;
use ensogl_text as text;



// =================
// === Constants ===
// =================

const HOVER_COLOR    : color::Rgba = color::Rgba::new(1.0,0.0,0.0,0.000_001);



// ==================
// === Text Label ===
// ==================

/// Text shape definition.
/// TODO consider generalising with `node::label`, but we want a different font size here, which
/// seems non-trivial to parametrise.
pub mod text {
    use super::*;

    const TEXT_FONT_SIZE : f32 = 9.0;
    // TODO: Char size based on values in `port.rs`. Should be calculated in based on actual font.
    const CHAR_WIDTH     : f32 = 7.224_609_4 * (TEXT_FONT_SIZE / 12.0);

    #[derive(Clone,CloneRef,Debug)]
    #[allow(missing_docs)]
    pub struct Shape {
        pub label : ensogl::display::shape::text::glyph::system::Line,
        pub obj   : display::object::Instance,

    }
    impl ensogl::display::shape::system::Shape for Shape {
        type System = ShapeSystem;
        fn sprites(&self) -> Vec<&Sprite> {
            vec![]
        }
    }
    impl display::Object for Shape {
        fn display_object(&self) -> &display::object::Instance {
            &self.obj
        }
    }
    impl Shape {
        /// Width of the text.
        pub fn width (&self) -> f32 {
            self.label.content().chars().count() as f32 * CHAR_WIDTH
        }
    }
    #[derive(Clone,CloneRef,Debug)]
    #[allow(missing_docs)]
    pub struct ShapeSystem {
        pub glyph_system: GlyphSystem,
        style_manager: StyleWatch,
    }
    impl ShapeSystemInstance for ShapeSystem {
        type Shape = Shape;

        fn new(scene:&Scene) -> Self {
            let style_manager = StyleWatch::new(&scene.style_sheet);
            let font          = scene.fonts.get_or_load_embedded_font("DejaVuSansMono").unwrap();
            let glyph_system  = GlyphSystem::new(scene,font);
            let symbol        = &glyph_system.sprite_system().symbol;
            scene.views.main.remove(symbol);
            scene.views.label.add(symbol);
            Self {glyph_system,style_manager} // .init_refresh_on_style_change()
        }

        fn new_instance(&self) -> Self::Shape {
            let color = color::Rgba::new(1.0,1.0,1.0,0.7);
            let obj   = display::object::Instance::new(Logger::new("test"));
            let label = self.glyph_system.new_line();
            label.set_font_size(TEXT_FONT_SIZE);
            label.set_font_color(color);
            label.set_text("");
            label.set_position_y(-0.25 * TEXT_FONT_SIZE);
            obj.add_child(&label);
            Shape {label,obj}
        }
    }
}





// ===============
// === Shapes  ===
// ===============

/// Invisible rectangular area that can be hovered.
pub mod hover_area {
    use super::*;

    ensogl::define_shape_system! {
        () {
            let width  : Var<Pixels> = "input_size.x".into();
            let height : Var<Pixels> = "input_size.y".into();
            let background    = Rect((&width,&height));
            let background    = background.fill(HOVER_COLOR);
            background.into()
        }
    }
}

/// Invisible rectangular area that can be hovered.
/// Note: needs to be an extra shape for sorting purposes.
pub mod chooser_hover_area {
    use super::*;

    ensogl::define_shape_system! {
        () {
            let width  : Var<Pixels> = "input_size.x".into();
            let height : Var<Pixels> = "input_size.y".into();
            let background    = Rect((&width,&height));
            let background    = background.fill(HOVER_COLOR);
            background.into()
        }
    }
}

/// Invisible rectangular area that can be hovered.
/// Note: needs to be an extra shape for sorting purposes.
pub mod background {
    use super::*;

    ensogl::define_shape_system! {
        () {
            let width  : Var<Pixels> = "input_size.x".into();
            let height : Var<Pixels> = "input_size.y".into();
            let radius             = node::NODE_SHAPE_RADIUS.px() ;
            let background_rounded = Rect((&width,&height)).corners_radius(&radius);
            let background_sharp   = Rect((&width,&height/2.0)).translate_y(-&height/4.0);
            let background         = background_rounded + background_sharp;
            let fill_color         = color::Rgba::from(color::Lcha::new(0.1,0.013,0.18,0.6));
            let background         = background.fill(fill_color);
            background.into()
        }
    }
}



// ===========
// === Frp ===
// ===========

ensogl_text::define_endpoints! {
    Input {
        set_label (String),
        set_size (Vector2),
        show_icons (),
        hide_icons (),
        set_visualization_alternatives (Vec<visualization::Path>),
    }
    Output {
        visualisation_selection  (Option<visualization::Path>),
        mouse_over   (),
        mouse_out    (),
    }
}



// ==============================
// === Quick Action Bar Model ===
// ==============================

#[derive(Clone,CloneRef,Debug)]
struct Model {
    hover_area                   : component::ShapeView<hover_area::Shape>,
    visualization_chooser        : visualization_chooser::VisualisationChooser,
    background                   : component::ShapeView<background::Shape>,
    visualisation_chooser_label  : text::Area,

    display_object        : display::object::Instance,

    size                  : Rc<Cell<Vector2>>,
}

impl QuickActionBarModel {
    fn new(scene:&Scene) -> Self {
        let logger                        = Logger::new("QuickActionBarModel");
        let background                    = component::ShapeView::new(&logger,scene);
        let hover_area                    = component::ShapeView::new(&logger,scene);
        let visualization_chooser_icon    = component::ShapeView::new(&logger,scene);
        let visualisation_chooser_label   = component::ShapeView::new(&logger,scene);
        let visualization_chooser_overlay = component::ShapeView::new(&logger,scene);
        let display_object                = display::object::Instance::new(&logger);
        let size                          = default();

        Model {
            hover_area,
            visualization_chooser,
            visualisation_chooser_label,
            display_object,
            size,
            background,
        }.init()
    }

    fn init(self) -> Self {
        self.add_child(&self.hover_area);

        self.set_label("None");

        self.visualisation_chooser_label.frp.set_default_color.emit(color::Rgba::new(1.0,1.0,1.0,1.0));

        // Remove default parent, then hide icons.
        self.show_quick_action_icons();
        self.hide_quick_action_icons();
        self
    }

    fn set_size(&self, size:Vector2) {
        self.size.set(size);
        self.hover_area.shape.size.set(size);
        self.background.shape.size.set(size);

        let height        = size.y;
        let width         = size.x;
        let right_padding = height / 2.0;
        self.visualization_chooser.frp.set_icon_size(Vector2::new(height,height));
        self.visualization_chooser.frp.set_icon_padding(Vector2::new(height/3.0,height/3.0));
        self.visualization_chooser.set_position_x((width/2.0) - right_padding);
        // self.visualization_chooser_overlay.shape.sprite.size.set(Vector2::new(width/2.0,height));
        // self.visualization_chooser_overlay.set_position_x(width/4.0);

        self.visualisation_chooser_label.set_position_y(0.25 * height);

    }

    fn set_label(&self, label:&str) {
        self.visualisation_chooser_label.shape.label.set_text(label);
        self.update_label_position()
    }

    fn show_quick_action_icons(&self) {
        self.add_child(&self.visualisation_chooser_label);
        self.add_child(&self.visualization_chooser_icon);
        self.add_child(&self.visualization_chooser_overlay);
        self.add_child(&self.background);
        self.add_child(&self.visualisation_chooser_label);

    }

    fn hide_quick_action_icons(&self) {
        self.visualization_chooser_icon.unset_parent();
        self.visualisation_chooser_label.unset_parent();
        self.visualization_chooser_overlay.unset_parent();
        self.background.unset_parent();
        self.visualisation_chooser_label.unset_parent();
        self.visualization_chooser.frp.hide_selection_menu.emit(());
    }
}

impl display::Object for Model {
    fn display_object(&self) -> &display::object::Instance {
        &self.display_object
    }
}



// ========================
// === Quick Action Bar ===
// ========================

/// UI for executing actions on a node. Consists of a number of interactive elements that are shown
/// when the area of the quick action bar is hovered and provides frp events when they are clicked.
///
/// Layout
/// ------
/// ```text
///     / ---------------------------- \
///    |                    <label> V   |
///    |--------------------------------|
///
/// TODO: will be extended with more quick action icons in #538
/// ```
#[derive(Clone,CloneRef,Debug)]
pub struct QuickActionBar {
    model   : Rc<Model>,
    /// Public FRP api.
    pub frp : Frp
}

impl QuickActionBar {

    /// Constructor.
    pub fn new(scene:&Scene) -> Self {
        let model = Rc::new(QuickActionBarModel::new(scene));
        let network = frp::Network::new();
        let frp = Frp::new(&network);
        QuickActionBar{model,network,frp}.init_frp()
    }

    fn init_frp(self) -> Self {
        let network = &self.frp.network;
        let frp     = &self.frp;
        let model   = &self.model;

        let hover_area             = &model.hover_area.events;
        let visualization_chooser  = &model.visualization_chooser.frp;

        frp::extend! { network

            eval frp.set_size ((size)   model.set_size(*size));
            eval frp.set_label ((label) model.set_label(label));

            eval frp.input.set_visualization_alternatives ((alternatives){
                visualization_chooser.input.set_alternatives.emit(alternatives);
            });

            eval_ frp.hide_icons ( model.hide() );
            eval_ frp.show_icons ( model.show() );



            any_component_over <- any(&hover_area.mouse_over,&visualization_chooser.icon_mouse_over);
            any_component_out  <- any(&hover_area.mouse_out,&visualization_chooser.icon_mouse_out);

            any_hovered <- source::<bool>();
            eval_ any_component_over ( any_hovered.emit(true) );
            eval_ any_component_out ( any_hovered.emit(false) );

            eval_ any_component_over (model.show());
            hide_icons  <- any_component_out.gate_not(&visualization_chooser.menu_open);
            hide_icons2 <- visualization_chooser.menu_closed.gate_not(&any_hovered);
            eval_ hide_icons (model.hide());
            eval_ hide_icons2 (model.hide());


            eval model.visualisation_chooser_label.width ((width) {
                model.visualisation_chooser_label.set_position_x(-width/2.0);
            });

            frp.source.visualisation_selection <+ visualization_chooser.selected_visualization;

        }

        self
    }
}

impl display::Object for QuickActionBar {
    fn display_object(&self) -> &display::object::Instance {
        &self.model.display_object()
    }
}