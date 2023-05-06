use std::
{
    collections::{HashMap}, hash::{Hasher, SipHasher}, rc::Rc, borrow::{BorrowMut, Borrow}, marker::PhantomData
};

#[cfg(feature="async")]
use std::sync::{RwLock, Arc};

#[cfg(feature="async")]
use std::sync;

#[cfg(feature="async")]
use std::thread;

use std::cell::RefCell;

use bytestream::{ByteOrder, StreamWriter};

/// Type alias to clarify that this number refers to a variable uniquely
pub type VariableIdentifier = u64;

pub type NativeFunctionBinding<State> = Box<dyn Fn(&VirtualMachine<State>, &StackFrame<State>) -> Result<(), &'static str>>;

pub struct FunctionParameter
{
    /// The name of the parameter
    pub name: String,

    /// The doc string
    pub doc: String
}

pub enum Function<State> where State: Clone
{
    NativeFunction {
        parameters: Vec<String>,
        binding: NativeFunctionBinding<State>
    },

    VirtualFunction {
        parameters: Vec<String>,
        instructions: InstructionSequence<State>
    }
}

impl<State> Function<State> where State: Clone
{
    pub fn call(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> Result<(), &'static str>
    {
        match self
        {
            Function::NativeFunction { parameters, binding } => {
                // It's up to the host function to figure out parameters here
                Ok((binding)(vm, frame)?)
            }

            // Execute virtual function code
            Function::VirtualFunction { parameters, instructions } => {
                Ok(vm.interpret(instructions)?)
            }
        }
    }
}

/// A namespace is a recursive structure used to store runtime generated data.
pub struct Namespace<'a, State> where State: Clone
{
    /// Child namespaces - used for enumeration
    #[cfg(not(feature="async"))]
    pub children: RefCell<HashMap<String, Namespace<'a, State>>>,

    /// All class definitions.
    #[cfg(not(feature="async"))]
    pub classes: RefCell<HashMap<String, ClassEntry<State>>>,

    /// A mapping of function name to function data
    #[cfg(not(feature="async"))]
    pub functions: RefCell<HashMap<String, Rc<Function<State>>>>,

    /// Child namespaces - used for enumeration
    #[cfg(feature="async")]
    pub children: Arc<RwLock<HashMap<String, Namespace<'a, State>>>>,

    /// All class definitions.
    #[cfg(feature="async")]
    pub classes: Arc<RwLock<HashMap<String, ClassEntry<State>>>>,

    /// A mapping of function name to function data
    #[cfg(feature="async")]
    pub functions: Arc<RwLock<HashMap<String, Arc<Function<State>>>>>,

    /// Lookup cache for function data
    pub function_cache: RefCell<HashMap<u64, Arc<Function<State>>>>
}

impl<State> Namespace<'_, State> where State: Clone
{
    #[cfg(not(feature="async"))]
    pub fn new() -> Self
    {
        return Self
        {
            children: RefCell::new(HashMap::new()),
            classes: RefCell::new(HashMap::new()),
            functions: RefCell::new(HashMap::new()),
            function_cache: RefCell::new(HashMap::new())
        };
    }

    #[cfg(feature="async")]
    pub fn new() -> Self
    {
        return Self
        {
            children: Arc::new(RwLock::new(HashMap::new())),
            classes: Arc::new(RwLock::new(HashMap::new())),
            functions: Arc::new(RwLock::new(HashMap::new())),
            function_cache: RefCell::new(HashMap::new())
        };
    }

    pub fn add_function_entry_slice(&mut self, function: Function<State>, path: &[String]) -> Result<(), &'static str>
    {
        // Need to descend more
        if path.len() > 1
        {
            let next_namespace_name = &path[0].to_lowercase();
            let next_slice = &path[1 ..];

            let mut namespace_write = self.children.write().unwrap();
            let namespace_lookup = namespace_write.get_mut(next_namespace_name);

            return match namespace_lookup {
                Some(next_namespace) => {
                    next_namespace.add_function_entry_slice(function, next_slice)
                },

                None => {
                    Err("Namespace Lookup Failed")
                }
            };
        }

        // We're at the final stop
        let function_name = &path[0].to_lowercase();
        let mut functions_write = self.functions.borrow_mut();

        #[cfg(not(feature="async"))]
        return match functions_write.insert(function_name.clone(), Rc::new(function))
        {
            Some(_insertion) => {
                Ok(()) // FIXME: Signal an overwrite
            },

            None => {
                Ok(())
            }
        };

        #[cfg(feature="async")]
        return match functions_write.write().unwrap().insert(function_name.clone(), Arc::new(function))
        {
            Some(_insertion) => {
                Ok(()) // FIXME: Signal an overwrite
            },

            None => {
                Ok(())
            }
        };
    }

    pub fn add_function_entry(&mut self, function: Function<State>, path: &Vec<String>) -> Result<(), &'static str>
    {
        return self.add_function_entry_slice(function, path.as_slice());
    }

    /// Performs a recursive search for a given function with no caching.
    pub fn lookup_function_uncached_slice(&self, path: &[String]) -> Result<Arc<Function<State>>, &'static str> // Result<Rc<Function<State>>, &'static str>
    {
        // Need to descend more
        if path.len() > 1
        {
            let next_namespace_name = &path[0].to_lowercase();
            let next_slice = &path[1 ..];
            
            #[cfg(not(feature="async"))]
            let namespace_read = self.children.borrow();

            #[cfg(feature="async")]
            let namespace_read = self.children.read().unwrap(); //.read().borrow().unwrap();

            let namespace_lookup = namespace_read.get(next_namespace_name);

            return match namespace_lookup {
                Some(next_namespace) => {
                    next_namespace.lookup_function_uncached_slice(next_slice)
                },

                None => {
                    Err("Namespace Lookup Failed")
                }
            };
        }

        // We're at the final stop
        let function_name = &path[0].to_lowercase();

        #[cfg(not(feature="async"))]
        let functions_read = self.functions.borrow();

        #[cfg(feature="async")]
        let functions_read = self.functions.read().unwrap();

        let function_lookup = functions_read.get(function_name);
        return match function_lookup {
            Some(found_function) => {
               Ok(found_function.clone())
            },

            None => {
                Err("Function Lookup Failed")
            }
        };
    }

    pub fn lookup_function_cached(&mut self, path: &Vec<String>) -> Result<Arc<Function<State>>, &'static str> //Result<Rc<Function<State>>, &'static str>
    {
        let mut hasher = SipHasher::new();
        for path_element in path.iter()
        {
            hasher.write(path_element.to_lowercase().as_bytes());
        }
        let lookup_id = hasher.finish();

        let cache_write = self.function_cache.borrow_mut();
        let cache_search = cache_write.get(&lookup_id);

        return match cache_search {
            Some(cache_hit) => {
                Ok(cache_hit.clone())
            },
            None => {
                let slow_search = self.lookup_function_uncached_slice(path.as_slice())?;
                Ok(slow_search)
            }
        };
    }

    pub fn lookup_function_uncached(&self, path: Vec<String>) -> Result<Arc<Function<State>>, &'static str> //Result<Rc<Function<State>>, &'static str>
    {
        return self.lookup_function_uncached_slice(path.as_slice());
    }
}

/// A virtual class in memory, used for typedefs
pub struct ClassEntry<State> where State: Clone
{
    pub name: String,
    pub namespaces: Vec<String>,

    pub functions: HashMap<String, Function<State>>
}

#[derive(Debug, Clone)]
pub enum AddressValue {
    RelativeOffset {
        offset: i32
    },

    AbsoluteTarget {
        index: usize
    }
}

#[derive(Debug, Clone)]
pub enum VariableReference<State> {
    Global {
        phantom: PhantomData<State>,
        value: VariableIdentifier
    },
    Local {
        phantom: PhantomData<State>,
        value: VariableIdentifier
    }
}

impl<State> VariableReference<State> where State: Clone
{
    #[inline(always)]
    fn perform_assignment(&self, vm: &VirtualMachine<State>, frame: &mut StackFrame<State>, rhs: &SystemValue<State>)
    {
        match self {
            VariableReference::Global { value, phantom } => {
                #[cfg(feature="async")]
                {
                    let mut globals_write = vm.globals.write().unwrap();
                    globals_write.insert((*value).clone(), rhs.as_raw(vm, frame));
                }

                #[cfg(not(feature="async"))]
                {
                    let mut globals_write = vm.globals.borrow_mut();
                    globals_write.insert((*value).clone(), rhs.as_raw(vm, frame));
                }
            },

            VariableReference::Local { value, phantom } => {
                frame.locals.insert((*value).clone(), rhs.as_raw(vm, frame));
            }
        }
    }

    /// Performs a variable lookup, returning a raw value read from memory
    #[inline(always)]
    pub fn deref(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> Result<RawValue<State>, &'static str>
    {
        return match self {
            VariableReference::Global { value, phantom } => {
                #[cfg(feature="async")]
                let globals_read = vm.globals.read().unwrap();

                #[cfg(not(feature="async"))]
                let globals_read = vm.globals.borrow_mut();

                match globals_read.get(value) {
                    Some(value) => {
                        Ok(value.clone())
                    },
                    None => {
                        Err("Variable Lookup Failed")
                    }
                }
            },
            VariableReference::Local { value, phantom } => {
                match frame.locals.get(value) {
                    Some(value) => {
                        Ok(value.clone())
                    },
                    None => {
                        Err("Variable Lookup Failed")
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FloatValue {
    pub value: f32
}

#[derive(Debug, Clone)]
pub struct IntegerValue {
    pub value: i32
}

#[derive(Debug, Clone)]
pub struct StringValue {
    pub value: String
}

#[derive(Debug, Clone)]
pub struct BooleanValue {
    pub value: bool
}

#[derive(Debug, Clone)]
pub struct VariableValue<State> {
    value: VariableReference<State>
}

// VariableSoftRef = Unresolved; requires a runtime lookup
// VariableHardRef = Resolved; memory can be read/write directly

/// Wrapper value representing a stored value in the virtual machine runtime.
#[derive(Debug, Clone)]
pub enum SystemValue<State> where State: Clone {
    Raw {
        value: RawValue<State>
    },

    Variable {
        value: VariableReference<State>
    }
}

impl<State> SystemValue<State> where State: Clone {
    #[inline(always)]
    pub fn as_raw(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> RawValue<State> {
        return match self {
            SystemValue::Raw { value } => {
                value.clone()
            },

            SystemValue::Variable { value } => {
                match value.deref(vm, frame) {
                    Ok(dereferenced) => {
                        dereferenced
                    },
                    Err(_) => {
                        // For now we mimic Torque where invalid lookups return ""
                        RawValue::String { 0: StringValue { value: "".to_owned() }}
                    }
                }
            }
        } 
    }

    #[inline(always)]
    pub fn as_variable(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> Result<VariableReference<State>, &'static str> {
        return match self {
            SystemValue::Raw { value: _ } => {
                Err("Not a Variable")
            },

            SystemValue::Variable { value } => {
                Ok(value.clone())
            }
        };
    }

    #[inline(always)]
    pub fn equals(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>, rhs: SystemValue<State>) -> bool {
        return self.as_raw(vm, frame).equals(vm, frame, &rhs.as_raw(vm, frame));
    }
}

#[derive(Debug, Clone)]
pub enum RawValue<State> where State: Clone {
    Float(FloatValue),
    Integer(IntegerValue),
    String(StringValue),
    Boolean(BooleanValue),
    Variable(VariableValue<State>)
}

impl<State> RawValue<State> where State: Clone {
    #[inline(always)]
    fn equals(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>, rhs: &RawValue<State>) -> bool {
        return self.as_float(vm, frame) == rhs.as_float(vm, frame);
    }

    #[inline(always)]
    pub fn as_string(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> String {
        return match self {
            RawValue::Float(value) => {
                (*value).value.to_string()
            },

            RawValue::Integer { 0: IntegerValue { value }} => {
                (*value).to_string()
            },

            RawValue::String { 0: StringValue { value }} => {
                (*value).clone()
            },

            RawValue::Boolean { 0: BooleanValue { value }} => {
                (*value).to_string()
            },

            RawValue::Variable { 0: VariableValue { value }} => {
                match value.deref(vm, frame) {
                    Ok(dereferenced) => {
                        dereferenced.as_string(vm, frame)
                    }
                    Err(_) => {
                        "".to_owned()
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn add(&self, rhs: &RawValue<State>, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> f32 {
        let lhs = self.as_float(vm, frame);
        let rhs = rhs.as_float(vm, frame);

        return lhs + rhs;
    }

    #[inline(always)]
    fn subtract(&self, rhs: &RawValue<State>, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> f32 {
        let lhs = self.as_float(vm, frame);
        let rhs = rhs.as_float(vm, frame);

        return lhs - rhs;
    }

    #[inline(always)]
    fn multiply(&self, rhs: &RawValue<State>, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> f32 {
        let lhs = self.as_float(vm, frame);
        let rhs = rhs.as_float(vm, frame);

        return lhs * rhs;
    }

    #[inline(always)]
    fn divide(&self, rhs: &RawValue<State>, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> f32 {
        let lhs = self.as_float(vm, frame);
        let rhs = rhs.as_float(vm, frame);

        return lhs / rhs;
    }

    #[inline(always)]
    fn negate(&mut self, vm: &mut VirtualMachine<State>, frame: &StackFrame<State>) {
        return match self {
            RawValue::Float(value) => {
                (*value).value = -value.value
            },

            RawValue::Integer { 0: IntegerValue { value }} => {
                *value = -(*value);
            },

            RawValue::String { 0: StringValue { value }} => {
                // FIXME
            },

            RawValue::Boolean { 0: BooleanValue { value }} => {
                *value = !(*value);
            },

            RawValue::Variable { 0: VariableValue { value }} => {
                // FIXME
            }
        }
    }

    #[inline(always)]
    pub fn as_float(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> f32 {
        return match self {
            RawValue::Float(value) => {
                value.value
            },

            RawValue::Integer { 0: IntegerValue {value }} => {
                *value as f32
            },

            RawValue::String { 0: StringValue { value }} => {
                match (*value).parse::<f32>() {
                    Ok(result) => {
                        result
                    }
                    Err(_) => {
                        0.0
                    }
                }
            },

            RawValue::Boolean { 0: BooleanValue { value }} => {
                if *value { 1.0 } else { 0.0 }
            },
            
            RawValue::Variable { 0: VariableValue { value }} => {
                match value.deref(vm, frame) {
                    Ok(dereferenced) => {
                        dereferenced.as_float(vm, frame)
                    }
                    Err(_) => {
                        0.0
                    }
                }      
            }
        } 
    }

    #[inline(always)]
    pub fn as_integer(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> i32 {
        return match self {
            RawValue::Float(value) => {
                value.value as i32
            },

            RawValue::Integer { 0: IntegerValue { value }} => {
                *value
            },

            RawValue::String { 0: StringValue { value }} => {
                match (*value).parse::<i32>() {
                    Ok(value) => {
                        value
                    },
                    Err(_) => {
                        0
                    }
                }
            },

            RawValue::Boolean { 0: BooleanValue { value }} => {
                if *value { 1 } else { 0 }
            },

            RawValue::Variable { 0: VariableValue { value }} => {
                match value.deref(vm, frame) {
                    Ok(dereferenced) => {
                        dereferenced.as_integer(vm, frame)
                    }
                    Err(_) => {
                        0
                    }
                }          
            }
        } 
    }

    #[inline(always)]
    pub fn as_boolean(&self, vm: &VirtualMachine<State>, frame: &StackFrame<State>) -> bool {  
        return match self {
            RawValue::Float(value) => {
                value.value != 0.0
            },

            RawValue::Integer { 0: IntegerValue { value }} => {
                (*value) != 0
            },

            RawValue::String { 0: StringValue { value }} => {
                match (*value).parse::<f32>() {
                    Ok(value) => {
                        value != 0.0
                    },
                    Err(_) => {
                        false
                    }
                }
            },

            RawValue::Boolean { 0: BooleanValue { value }} => {
                *value
            },

            RawValue::Variable { 0: VariableValue { value }} => {
                // FIXME: Hardcoded
                true
            }
        }
    }
}


pub struct PushFloat
{
    pub value: f32
}

pub enum OpCode<State>
{
    // General state management
    PushFloat(PushFloat),

    PushInteger {
        value: i32
    },
    PushString {
        value: String
    },
    Pop {

    },
    Jump {
        target: AddressValue
    },
    JumpTrue {
        target: AddressValue
    },
    JumpFalse {
        target: AddressValue
    },
    NOP {

    },

    // Crappy workaround op for testing
    Swap {

    },

    Assignment {

    },
    Concat {

    },
    Negate { 

    },
    Not { 

    },
    CallFunction {
        target: Vec<String>
    },

    // Logical Instructions
    LogicalAnd {

    },
    LogicalOr {

    },

    // Bitwise Instructions
    BitwiseAnd {

    },
    BitwiseOr {

    },

    // Arithmetic
    Add {

    },
    Minus {

    },
    Modulus {

    },
    Multiply {

    },
    Divide {

    },

    // Relational
    LessThan {

    },
    GreaterThan {

    },
    GreaterThanOrEqual {

    },
    Equals { 

    },
    NotEquals {

    },
    StringEquals {

    },
    StringNotEqual {

    },

    PushVariable {
        variable: VariableReference<State>
    }
}

impl<State> OpCode<State>
{
    fn get_type(&self) -> String
    {
        return match self {
            OpCode::PushFloat (value) => "Error".to_owned(),

            OpCode::PushInteger { value } => "Error".to_owned(),
            OpCode::PushString { value } => "Error".to_owned(),
            OpCode::Pop {  } => "Error".to_owned(),
            OpCode::Jump { target } => "Error".to_owned(),
            OpCode::JumpTrue { target } => "Error".to_owned(),
            OpCode::JumpFalse { target } => "Error".to_owned(),
            OpCode::NOP {  } => "Error".to_owned(),
            OpCode::Swap {  } => "Error".to_owned(),
            OpCode::Assignment {  } => "Error".to_owned(),
            OpCode::Concat {  } => "Error".to_owned(),
            OpCode::Negate {  } => "Error".to_owned(),
            OpCode::Not {  } => "Error".to_owned(),
            OpCode::CallFunction { target } => "Error".to_owned(),
            OpCode::LogicalAnd {  } => "Error".to_owned(),
            OpCode::LogicalOr {  } => "Error".to_owned(),
            OpCode::BitwiseAnd {  } => "Error".to_owned(),
            OpCode::BitwiseOr {  } => "Error".to_owned(),
            OpCode::Add {  } => "Error".to_owned(),
            OpCode::Minus {  } => "Error".to_owned(),
            OpCode::Modulus {  } => "Error".to_owned(),
            OpCode::Multiply {  } => "Error".to_owned(),
            OpCode::Divide {  } => "Error".to_owned(),
            OpCode::LessThan {  } => "Error".to_owned(),
            OpCode::GreaterThan {  } => "Error".to_owned(),
            OpCode::GreaterThanOrEqual {  } => "Error".to_owned(),
            OpCode::Equals {  } => "Error".to_owned(),
            OpCode::NotEquals {  } => "Error".to_owned(),
            OpCode::StringEquals {  } => "Error".to_owned(),
            OpCode::StringNotEqual {  } => "Error".to_owned(),
            OpCode::PushVariable { variable } => "Error".to_owned()
        };
    }
}

//type InstructionSequence = Vec<OpCode>;
pub struct InstructionSequence<State>
{
    pub ops: Vec<OpCode<State>>
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

impl<State> InstructionSequence<State>
{
    pub fn serialize(&self)
    {
        let mut buffer = Vec::<u8>::new();
        buffer.reserve(2048);
        
        for op in self.ops.iter()
        {
            let type_name = op.get_type();

            // FIXME: Unstable feature here? We need to ensure the hash algorithm remains static
            let mut hasher = SipHasher::new();
            hasher.write(type_name.as_bytes());
            let opcode_id = hasher.finish();

            println!("OP: {}", opcode_id);

            opcode_id.write_to(&mut buffer, ByteOrder::LittleEndian).unwrap();
        }
        println!("Done");
    }
}

pub struct StackFrame<State> where State: Clone
{
    /// Current VM thread-local stack state
    #[cfg(not(feature="register-vm"))]
    pub stack: Vec<SystemValue<State>>,

    /// Local thread-local variables allocated at this frame
    pub locals: HashMap<VariableIdentifier, RawValue<State>>
}

pub struct VirtualMachine<'a, State> where State: Clone
{
    /// A mapping of global string identifiers to their value
    #[cfg(feature="async")]
    pub globals: Arc<RwLock<HashMap<VariableIdentifier, RawValue<State>>>>,

    /// A mapping of global string identifiers to their value
    #[cfg(not(feature="async"))]
    pub globals: RefCell<HashMap<VariableIdentifier, RawValue<State>>>,

    /// Root namespaces
    pub root_namespace: RefCell<Namespace<'a, State>>,

    /// Application state, user provided
    pub state: State
}

#[inline(always)]
fn process_address(offset_out: &mut usize, address: &AddressValue)
{
    match address {
        AddressValue::RelativeOffset { offset } => {
            // FIXME: Duplicate code
            if *offset < 0 {
                let calculated_offset = (*offset * -1) as usize;
                *offset_out -= calculated_offset;
            }
            else {
                let calculated_offset = (*offset * -1) as usize;
                *offset_out += calculated_offset;
            }
        },
        AddressValue::AbsoluteTarget { index } => {
            *offset_out = *index;
        }
    }
}

impl<State> VirtualMachine<'_, State> where State: Clone
{
    #[inline(always)]
    #[cfg(feature="async")]
    pub fn new(state: State) -> Self {
        let globals: Arc<RwLock<HashMap<VariableIdentifier, RawValue<State>>>> = Arc::new(RwLock::new(HashMap::new()));
        let mut globals_write = globals.write().unwrap();
        globals_write.reserve(1024);
        drop(globals_write);

        return Self {
            globals: globals,
            state: state,
            root_namespace: RefCell::new(Namespace::new()),
        };
    }

    #[inline(always)]
    #[cfg(not(feature="async"))]
    pub fn new(state: State) -> Self {
        let mut globals: HashMap<VariableIdentifier, RawValue<State>> = HashMap::new();
        globals.reserve(1024);

        return Self {
            root_namespace: RefCell::new(Namespace::new()),
            globals: RefCell::new(globals),
            state: state
        };
    }
    
    pub fn interpret(&self, instructions: &InstructionSequence<State>) -> Result<(), &'static str>
    {
        // Allocate new frame
        let mut stack: Vec<SystemValue<State>> = Vec::new();
        stack.reserve(1024);
        let mut frame = StackFrame {
            locals: HashMap::new(),
            stack: stack
        };
        
        let mut continue_running: bool = true;
        let mut current_index: usize = 0;
        
        // Ensure the total number of ops is read once and cached
        let op_count = instructions.ops.len();

        loop {
            if !continue_running || current_index >= op_count {
                return Ok(());
            }

            // Looks like this might be slightly faster than indexing
            let current_instruction = instructions.ops.get(current_index).unwrap();

            // By default we increment the index but some ops can override this
            current_index += 1;
            match current_instruction {
                OpCode::Swap {} => {  
                    let lhs = frame.stack.pop();
                    let rhs = frame.stack.pop();
                    
                    #[cfg(feature="fault-checks")]
                    if lhs.is_none() || rhs.is_none()
                    {
                        return Err("Failed to load Values off Stack for Swap");
                    }

                    frame.stack.push(rhs.unwrap());
                    frame.stack.push(lhs.unwrap());
                },
                OpCode::PushFloat (value) => {
                    frame.stack.push(SystemValue::Raw { value: RawValue::Float { 0: FloatValue { value: value.value }}});
                },
                OpCode::PushInteger { value } => {
                    frame.stack.push(SystemValue::Raw { value: RawValue::Integer { 0: IntegerValue { value: *value }}});
                },
                OpCode::PushString { value } => {
                    // NOTE: Continuous string alloc here
                    frame.stack.push(SystemValue::Raw { value: RawValue::String { 0: StringValue { value: value.to_string() }}});
                },
                OpCode::Pop {  } => {
                    // For now we let the application halt if stack is empty
                    let pop_result = frame.stack.pop();

                    #[cfg(feature="fault-checks")]
                    if pop_result.is_none() {
                        return Err("Failed to Pop Value from Stack");
                    }
                    pop_result.unwrap();
                },
                OpCode::Jump { target } => {
                    process_address(&mut current_index, target);
                },
                OpCode::JumpTrue { target } => {
                    let current_value = frame.stack.pop();

                    #[cfg(feature="fault-checks")]
                    if current_value.is_none() {
                        return Err("Failed to Load condition for JumpTrue from Stack");
                    }

                    if current_value.unwrap().as_raw(self, &frame).as_boolean(self, &frame) {
                        process_address(&mut current_index, target);
                    }
                },
                OpCode::JumpFalse { target } => {
                    let current_value = frame.stack.pop();

                    #[cfg(feature="fault-checks")]
                    if current_value.is_none() {
                        return Err("Failed to Load condition for JumpTrue from Stack");
                    }

                    if !current_value.unwrap().as_raw(self, &frame).as_boolean(self, &frame) {
                        process_address(&mut current_index, target);
                    }
                },
                OpCode::NOP {  } => {

                },
                OpCode::Assignment {  } => {
                    let lhs = frame.stack.pop();
                    let rhs = frame.stack.pop();

                    #[cfg(feature="fault-checks")]
                    if lhs.is_none() || rhs.is_none() {
                        return Err("Failed to Load lhs & rhs from Stack for Assignment");
                    }

                    let lhs_unwrapped = lhs.unwrap();

                    // FIXME: Assuming variable lookup succeeds
                    lhs_unwrapped.as_variable(self, &frame).unwrap().perform_assignment(self, &mut frame, &rhs.unwrap());

                    frame.stack.push(lhs_unwrapped); // Push a reference to current variable back to stack
                },
                OpCode::Concat {  } => {
                    let lhs = frame.stack.pop();
                    let rhs = frame.stack.pop();

                    #[cfg(feature="fault-checks")]
                    if lhs.is_none() || rhs.is_none() {
                        return Err("Failed to Load lhs & rhs from Stack for Concat");
                    }

                    let mut result = lhs.unwrap().as_raw(self, &frame).as_string(self, &frame);
                    result.push_str(&rhs.unwrap().as_raw(self, &frame).as_string(self, &frame));

                    frame.stack.push(SystemValue::Raw { value: RawValue::String { 0: StringValue { value: result }}});
                },
                OpCode::Negate {  } => {
                    panic!("Not Implemented");
                     let current_value = frame.stack.last_mut().unwrap();
                },
                OpCode::Not {  } => {
                    let current_value = frame.stack.pop().unwrap();
                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: !current_value.as_raw(self, &frame).as_boolean(self, &frame) }}});
                },
                OpCode::CallFunction { target } => {
                    let mut namespace_write = self.root_namespace.borrow_mut();
                    let function_lookup = namespace_write.lookup_function_cached(target).unwrap();
                    function_lookup.call(self, &frame).unwrap();
                },
                OpCode::LogicalAnd {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: lhs.as_raw(self, &frame).as_boolean(self, &frame) && rhs.as_raw(self, &frame).as_boolean(self, &frame) }}});
                },
                OpCode::LogicalOr {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: lhs.as_raw(self, &frame).as_boolean(self, &frame) || rhs.as_raw(self, &frame).as_boolean(self, &frame) }}});
                },
                OpCode::BitwiseAnd {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Integer { 0: IntegerValue { value: lhs.as_raw(self, &frame).as_integer(self, &frame) & rhs.as_raw(self, &frame).as_integer(self, &frame) }}});
                },
                OpCode::BitwiseOr {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();
                    
                    frame.stack.push(SystemValue::Raw { value: RawValue::Integer { 0: IntegerValue { value: lhs.as_raw(self, &frame).as_integer(self, &frame) | rhs.as_raw(self, &frame).as_integer(self, &frame) }}});
                },
                OpCode::Add {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    let result = lhs.as_raw(self, &frame).add(&rhs.as_raw(self, &frame), self, &frame);
                    frame.stack.push(SystemValue::Raw { value: RawValue::Float { 0: FloatValue { value: result }}});
                },
                OpCode::Minus {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    let result = lhs.as_raw(self, &frame).subtract(&rhs.as_raw(self, &frame), self, &frame);

                    frame.stack.push(SystemValue::Raw { value: RawValue::Float { 0: FloatValue { value: result }}});
                },
                OpCode::Modulus {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Integer { 0: IntegerValue { value: lhs.as_raw(self, &frame).as_integer(self, &frame) % rhs.as_raw(self, &frame).as_integer(self, &frame) }}});
                },
                OpCode::Multiply {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    let result = lhs.as_raw(self, &frame).multiply(&rhs.as_raw(self, &frame), self, &frame);
                    frame.stack.push(SystemValue::Raw { value: RawValue::Float { 0: FloatValue { value: result }}});
                },
                OpCode::Divide {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    let result = lhs.as_raw(self, &frame).divide(&rhs.as_raw(self, &frame), self, &frame);
                    
                    frame.stack.push(SystemValue::Raw { value: RawValue::Float { 0: FloatValue { value: result }}});
                },
                OpCode::LessThan {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: lhs.as_raw(self, &frame).as_float(self, &frame) < rhs.as_raw(self, &frame).as_float(self, &frame) }}});
                },
                OpCode::GreaterThan {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: lhs.as_raw(self, &frame).as_float(self, &frame) > rhs.as_raw(self, &frame).as_float(self, &frame) }}});
                },
                OpCode::GreaterThanOrEqual {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: lhs.as_raw(self, &frame).as_float(self, &frame) >= rhs.as_raw(self, &frame).as_float(self, &frame) }}});
                },
                OpCode::Equals {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: lhs.equals(self, &frame, rhs) }}});
                },
                OpCode::NotEquals {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: !lhs.equals(self, &frame, rhs) }}});
                },
                OpCode::StringEquals {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: lhs.as_raw(self, &frame).as_string(self, &frame) == rhs.as_raw(self, &frame).as_string(self, &frame) }}});
                },
                OpCode::StringNotEqual {  } => {
                    let lhs = frame.stack.pop().unwrap();
                    let rhs = frame.stack.pop().unwrap();

                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: lhs.as_raw(self, &frame).as_string(self, &frame) != rhs.as_raw(self, &frame).as_string(self, &frame) }}});
                },
                OpCode::PushVariable { variable } => {
                    frame.stack.push(SystemValue::Variable { value: variable.clone() });
                }
            }
        }
    }
}