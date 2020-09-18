
use enso_prelude::*;
use wasm_bindgen::prelude::*;
use ensogl_system_web as web;
use enso_automata::*;

use enso_frp as frp;

use frp::io::keyboard2::Keyboard;
use frp::io::keyboard2 as keyboard;

pub use logger;
pub use logger::*;
pub use logger::AnyLogger;
pub use logger::disabled::Logger;


fn hash<T:Hash>(t:&T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}


fn print_matrix(matrix:&data::Matrix<dfa::State>) {
    println!("rows x cols = {} x {} ({})",matrix.rows, matrix.columns,matrix.matrix.len());
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            let elem = matrix.safe_index(row,column).unwrap();
            let repr = if elem.is_invalid() { "-".into() } else { format!("{}",elem.id()) };
            print!("{} ",repr);
        }
        println!("");
    }
}



#[wasm_bindgen]
#[allow(dead_code)]
pub fn entry_point_shortcuts() {
    web::forward_panic_hook_to_console();
    web::set_stdout();
    web::set_stack_trace_limit();

    main();

}



fn reverse_key(key:&str) -> String {
    format!("-{}",key)
}

/// List of special keys. Special keys can be grouped together to distinguish action sequences like
/// `ctrl + a` and `a + ctrl`. Please note, that this is currently not happening.
const SPECIAL_KEYS : &'static [&'static str] = &["ctrl","alt","meta","cmd","shift"];


const DOUBLE_EVENT_TIME_MS : f32 = 500.0;

// ==================
// === ActionType ===
// ==================

/// The type of the action. Could be applied to keyboard, mouse, or any mix of input events.
#[derive(Clone,Copy,Debug,Eq,Hash,PartialEq)]
pub enum ActionType {
    Press, Release, DoublePress, DoubleClick
}
pub use ActionType::*;



// =================
// === Shortcuts ===
// =================

#[derive(Debug)]
pub struct Shortcuts {
    dirty         : bool,
    nfa           : Nfa,
    dfa           : Dfa,
    states        : HashMap<String,nfa::State>,
    connections   : HashSet<(nfa::State,nfa::State)>,
    always_state  : nfa::State,
    current       : dfa::State,
    pressed       : HashSet<String>,
    action_map    : HashMap<ActionType,HashMap<nfa::State,String>>,
    press_times   : HashMap<dfa::State,f32>,
    release_times : HashMap<dfa::State,f32>,
}

impl Shortcuts {
    /// Constructor.
    pub fn new() -> Self {
        let mut nfa       = Nfa::default();
        let dfa           = Dfa::from(&nfa);
        let states        = default();
        let connections   = default();
        let always_state  = nfa.new_pattern(nfa.start,Pattern::any().many());
        let current       = Dfa::START_STATE;
        let pressed       = default();
        let dirty         = true;
        let action_map    = default();
        let press_times   = default();
        let release_times = default();
        Self {dirty,nfa,dfa,states,connections,always_state,current,pressed,action_map,press_times
             ,release_times}
    }


    pub fn add(&mut self, action_type:ActionType, expr:impl AsRef<str>, action:impl Into<String>) {
        self.dirty = true;

        let special_keys : HashSet<&str> = SPECIAL_KEYS.iter().map(|t|*t).collect();
        let expr = expr.as_ref();

        let end_state = if expr.starts_with('-') {
            let key = format!("-{}",expr[1..].trim().to_lowercase());
            let sym = Symbol::new_named(hash(&key),key);
            let pat = Pattern::symbol(&sym);
            self.nfa.new_pattern(self.always_state,pat)
        } else {
            let mut special = vec![];
            let mut normal  = vec![];

            for chunk in expr.split('+').map(|t| t.trim()) {
                let map = if special_keys.contains(chunk) { &mut special } else { &mut normal };
                map.push(chunk);
            }

            let mut all = special.clone();
            all.extend(&normal);
            self.add_key_permutations(self.nfa.start, &all)
        };
        self.action_map.entry(action_type).or_default().insert(end_state,action.into());
    }

    pub fn add_key_permutations(&mut self, source:nfa::State, keys:&[&str]) -> nfa::State {
        self.dirty = true;

        let len = keys.len();
        let mut state = source;

        for perm in keys.iter().permutations(len) {
            state = source;
            let mut path  = vec![];
            let mut repr  = String::new();
            for key in perm {
                path.push(*key);
                path.sort();
                repr  = path.join(" ");
                state = match self.states.get(&repr) {
                    Some(&target) => {
                        self.bidirectional_connect(state,target,key.to_string());
                        target
                    },
                    None => {
                        let out = self.bidirectional_pattern(state,key.to_string());
                        self.states.insert(repr.clone(),out);
                        out
                    }
                }
            }
        }
        state
    }

    fn bidirectional_connect(&mut self, source:nfa::State, target:nfa::State, key:String) {
        if !self.connections.contains(&(source,target)) {
            self.connections.insert((source,target));
            let key_r = reverse_key(&key);
            let sym   = Symbol::new_named(hash(&key),key);
            let sym_r = Symbol::new_named(hash(&key_r),key_r);
            self.nfa.connect_via(source,target,&(sym.clone()..=sym));
            self.nfa.connect_via(target,source,&(sym_r.clone()..=sym_r));
        }
    }

    fn bidirectional_pattern(&mut self, source:nfa::State, key:String) -> nfa::State {
        let key_r  = reverse_key(&key);
        let sym    = Symbol::new_named(hash(&key),key);
        let sym_r  = Symbol::new_named(hash(&key_r),key_r);
        let pat    = Pattern::symbol(&sym);
        let target = self.nfa.new_pattern(source,pat);
        self.nfa.connect_via(target,source,&(sym_r.clone()..=sym_r));
        self.connections.insert((source,target));
        target
    }

    /// Process the press input event. See `on_event` docs to learn more.
    pub fn on_press(&mut self, input:&str) -> Vec<String> {
        self.on_event(input,true)
    }

    /// Process the release input event. See `on_event` docs to learn more.
    pub fn on_release(&mut self, input:&str) -> Vec<String> {
        self.on_event(input,false)
    }

    /// Process the `input` event. Events are strings uniquely identifying the source of the event,
    /// like "key_a", or "mouse_button_1". The `press` parameter is set to true if it was a press
    /// event, and is set to false in case of a release event.
    fn on_event(&mut self, input:&str, press:bool) -> Vec<String> {
        self.recompute_on_dirty();
        let action        = if press { Press }       else { Release };
        let double_action = if press { DoublePress } else { DoubleClick };
        let input         = input.to_lowercase();
        let symbol_input  = if press { input.clone() } else { format!("-{}",input) };
        let symbol        = Symbol::from(hash(&symbol_input));
        let current_state = self.current;
        let next_state    = self.dfa.next_state(current_state,&symbol);
        let focus_state   = if press { next_state } else { current_state };
        let nfa_states    = &self.dfa.sources[focus_state.id()];
        let time : f32    = web::performance().now() as f32;
        let last_time_map = if press { &self.press_times } else { &self.release_times };
        let last_time     = last_time_map.get(&focus_state);
        let time_diff     = last_time.map(|t| time-t);
        let is_double     = time_diff.map(|t| t < DOUBLE_EVENT_TIME_MS) == Some(true);
        let new_time      = if is_double { 0.0 } else { time };
        self.current      = next_state;
        let mut actions   = nfa_states.iter().filter_map(|t|self.get_action(action,*t)).collect_vec();
        if is_double {
            actions.extend(nfa_states.iter().filter_map(|t|self.get_action(double_action,*t)));
        }
        if press {
            self.pressed.insert(input);
            self.press_times.insert(focus_state,new_time);
        } else {
            self.pressed.remove(&input);
            self.release_times.insert(focus_state,new_time);
            if self.pressed.is_empty() {
                self.current = Dfa::START_STATE;
            }
            self.reset_to_known_state();
        }
        actions
    }

    pub fn reset_to_known_state(&mut self) {
        if self.current.is_invalid() {
            let path = self.pressed.iter().sorted().cloned().collect_vec();
            self.current = Dfa::START_STATE;
            for p in path {
                self.current = self.dfa.next_state(self.current, &Symbol::from(hash(&p)));
            }
        }
    }

    fn get_action(&self, action_type:ActionType, state:nfa::State) -> Option<String> {
        self.action_map.get(&action_type).and_then(|m|m.get(&state).cloned())
    }

    fn recompute_on_dirty(&mut self) {
        if self.dirty {
            self.dirty   = false;
            self.dfa     = (&self.nfa).into();
            self.pressed = default();
        }
    }
}


pub fn main() {


    let sym_mouse_0 = Symbol::from(hash(&"mouse_0".to_string()));
    let sym_mouse_1 = Symbol::from(hash(&"mouse_1".to_string()));
    let sym_mouse_2 = Symbol::from(hash(&"mouse_2".to_string()));
    let sym_mouse_3 = Symbol::from(hash(&"mouse_3".to_string()));
    let sym_mouse_4 = Symbol::from(hash(&"mouse_4".to_string()));
    let sym_ctrl = Symbol::from(hash(&"ctrl".to_string()));
    let sym_a = Symbol::from(hash(&"a".to_string()));
    let sym_a_r = Symbol::from(hash(&"release a".to_string()));
    let sym_b = Symbol::from(hash(&"b".to_string()));
    let sym_b_r = Symbol::from(hash(&"release b".to_string()));
    let sym_c = Symbol::from(hash(&"c".to_string()));
    let sym_x = Symbol::from(hash(&"o".to_string()));

    let pat_ctrl    = Pattern::symbol(&sym_ctrl);
    let pat_a       = Pattern::symbol(&sym_a);
    let pat_a_r     = Pattern::symbol(&sym_a_r);
    let pat_b       = Pattern::symbol(&sym_b);
    let pat_b_r     = Pattern::symbol(&sym_b_r);
    let pat_mouse_0 = Pattern::symbol(&sym_mouse_0);
    let pat_mouse_1 = Pattern::symbol(&sym_mouse_1);
    let pat_mouse_2 = Pattern::symbol(&sym_mouse_2);
    let pat_mouse_3 = Pattern::symbol(&sym_mouse_3);
    let pat_mouse_4 = Pattern::symbol(&sym_mouse_4);

    let pat_any_mouse = pat_mouse_0 | pat_mouse_1 | pat_mouse_2 | pat_mouse_3 | pat_mouse_4;


    let mut s = Shortcuts::new();

    s.add(DoublePress, "meta + a", "hello");
    // s.add("meta + b");

    // s.add("meta + ctrl + alt + space + a1");
    // s.add("meta + ctrl + alt + space + a2");

    // s.add("- c");


    println!("---------");
    s.recompute_on_dirty();
    let elems = s.dfa.links.matrix.len() as f32;
    let bytes = elems * 8.0;
    let mb = bytes / 1000000.0;
    println!("!!!o {}",mb);
    // print_matrix(&s.dfa.links);


    // let mut nfa = Nfa::new();
    // let p = (Pattern::any()).many();
    // nfa.new_pattern(nfa.start,p);
    // // nfa.new_pattern(nfa.start,Pattern::symbol(&Symbol::from(hash(&"p".to_string()))));
    // let dfa = Dfa::from(&nfa);
    //
    // println!("??? {:?}", dfa.next_state(Dfa::START_STATE,&Symbol::from(hash(&"p".to_string()))));
    //
    // println!("{}",nfa.visualize());
    // print_matrix(&dfa.links);



    println!("{}",s.nfa.visualize());

    println!("---------");

    // println!("{}",s.dfa.visualize());

    let ss = Rc::new(RefCell::new(s));

    let logger = Logger::new("kb");
    let kb = Keyboard::new();
    let bindings = keyboard::DomBindings::new(&logger,&kb);

    let ss2 = ss.clone_ref();
    let ss3 = ss.clone_ref();
    frp::new_network! { network
        foo <- kb.down.map(move |t| ss2.borrow_mut().on_press(t.simple_name()));
        trace foo;
        foo <- kb.up.map(move |t| ss3.borrow_mut().on_release(t.simple_name()));
        trace foo;
    }
    mem::forget(network);
    mem::forget(bindings);
    mem::forget(ss);


}




// - shift left left
//
// any_key (?mouse)
//
// left (?mouse)
//
// key? left_down
//
// ctr -> b -> release ctrl
//
// any letter WITHOUT modifiers (typing but not cmd+a)


// +lmb           - start selection
// -lmb (ANY key) - stop selection