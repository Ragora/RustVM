use std::{time::{Instant}, collections::{HashMap}, hash::{SipHasher}};

use std::hash::{Hasher};

#[cfg(feature="async")]
use std::sync::{RwLock, Arc};

#[cfg(feature="async")]
use std::sync;

#[cfg(feature="async")]
use std::thread;

use std::cell::RefCell;

use bytestream::{ByteOrder, StreamWriter};

/// Type alias to clarify that this number refers to a variable uniquely
type VariableIdentifier = u64;

#[inline(always)]
pub fn variable_name_to_identifier(name: String) -> VariableIdentifier
{
    // Case insensitive
    let processed_string = name.to_lowercase();

    // FIXME: Unstable feature here? We need to ensure the hash algorithm remains static
    let mut hasher = SipHasher::new();
    hasher.write(processed_string.as_bytes());
    return hasher.finish();
}

#[derive(Debug, Clone)]
pub enum AddressType {
    RelativeOffset {
        offset: i32
    },

    AbsoluteTarget {
        index: usize
    }
}

#[derive(Debug, Clone)]
pub enum VariableReference {
    Global {
        value: VariableIdentifier
    },
    Local {
        value: VariableIdentifier
    }
}

impl VariableReference
{
    #[inline(always)]
    fn perform_assignment(&self, vm: &VirtualMachine, frame: &mut StackFrame, rhs: &SystemValue)
    {
        match self {
            VariableReference::Global { value } => {
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

            VariableReference::Local { value } => {
                frame.locals.insert((*value).clone(), rhs.as_raw(vm, frame));
            }
        }
    }

    /// Performs a variable lookup, returning a raw value read from memory
    #[inline(always)]
    pub fn deref(&self, vm: &VirtualMachine, frame: &StackFrame) -> RawValue
    {
        return match self {
            VariableReference::Global { value } => {
                #[cfg(feature="async")]
                let globals_read = vm.globals.read().unwrap();

                #[cfg(not(feature="async"))]
                let globals_read = vm.globals.borrow_mut();

                match globals_read.get(value) {
                    Some(value) => {
                        value.clone()
                    },
                    None => {
                        // For now we mimic Torque where invalid lookups return ""
                        RawValue::String { 0: StringValue { value: "".to_owned() }}
                    }
                }
            },
            VariableReference::Local { value } => {
                match frame.locals.get(value) {
                    Some(value) => {
                        value.clone()
                    },
                    None => {
                        // For now we mimic Torque where invalid lookups return ""
                        RawValue::String { 0: StringValue { value: "".to_owned() }}
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
pub struct VariableValue {
    value: VariableReference
}

#[derive(Debug, Clone)]
pub enum RawValue {
    Float(FloatValue),
    Integer(IntegerValue),
    String(StringValue),
    Boolean(BooleanValue),
    Variable(VariableValue)
}

#[derive(Debug, Clone)]
pub enum SystemValue {
    Raw {
        value: RawValue
    },

    Variable {
        value: VariableReference
    }
}

impl SystemValue {
    #[inline(always)]
    pub fn as_raw(&self, vm: &VirtualMachine, frame: &StackFrame) -> RawValue {
        return match self {
            SystemValue::Raw { value } => {
                value.clone()
            },

            SystemValue::Variable { value } => {
                value.deref(vm, frame).clone()
            }
        } 
    }

    #[inline(always)]
    pub fn as_variable(&self, vm: &VirtualMachine, frame: &StackFrame) -> Result<VariableReference, &'static str> {
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
    pub fn equals(&self, vm: &VirtualMachine, frame: &StackFrame, rhs: SystemValue) -> bool {
        return self.as_raw(vm, frame).equals(vm, frame, &rhs.as_raw(vm, frame));
    }
}

impl RawValue {
    #[inline(always)]
    fn equals(&self, vm: &VirtualMachine, frame: &StackFrame, rhs: &RawValue) -> bool {
        return self.as_float(vm, frame) == rhs.as_float(vm, frame);
    }

    #[inline(always)]
    fn as_string(&self, vm: &VirtualMachine, frame: &StackFrame) -> String {
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
                value.deref(vm, frame).as_string(vm, frame)
            }
        }
    }

    #[inline(always)]
    fn add(&self, rhs: &RawValue, vm: &VirtualMachine, frame: &StackFrame) -> f32 {
        let lhs = self.as_float(vm, frame);
        let rhs = rhs.as_float(vm, frame);

        return lhs + rhs;
    }

    #[inline(always)]
    fn subtract(&self, rhs: &RawValue, vm: &VirtualMachine, frame: &StackFrame) -> f32 {
        let lhs = self.as_float(vm, frame);
        let rhs = rhs.as_float(vm, frame);

        return lhs - rhs;
    }

    #[inline(always)]
    fn multiply(&self, rhs: &RawValue, vm: &VirtualMachine, frame: &StackFrame) -> f32 {
        let lhs = self.as_float(vm, frame);
        let rhs = rhs.as_float(vm, frame);

        return lhs * rhs;
    }

    #[inline(always)]
    fn divide(&self, rhs: &RawValue, vm: &VirtualMachine, frame: &StackFrame) -> f32 {
        let lhs = self.as_float(vm, frame);
        let rhs = rhs.as_float(vm, frame);

        return lhs / rhs;
    }

    #[inline(always)]
    fn negate(&mut self, vm: &mut VirtualMachine, frame: &StackFrame) {
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
    fn as_float(&self, vm: &VirtualMachine, frame: &StackFrame) -> f32 {
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
                value.deref(vm, frame).as_float(vm, frame)         
            }
        } 
    }

    #[inline(always)]
    fn as_integer(&self, vm: &VirtualMachine, frame: &StackFrame) -> i32 {
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
                value.deref(vm, frame).as_integer(vm, frame)           
            }
        } 
    }

    #[inline(always)]
    fn as_boolean(&self, vm: &VirtualMachine, frame: &StackFrame) -> bool {  
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

pub enum OpCode
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
        target: AddressType
    },
    JumpTrue {
        target: AddressType
    },
    JumpFalse {
        target: AddressType
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


    PushLocalReference {
        variable: VariableReference
    },
    PushGlobalReference {
        variable: VariableReference
    }
}

impl OpCode
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
            OpCode::CallFunction {  } => "Error".to_owned(),
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
            OpCode::PushLocalReference { variable } => "Error".to_owned(),
            OpCode::PushGlobalReference { variable } => "Error".to_owned()
        };
    }
}

//type InstructionSequence = Vec<OpCode>;
pub struct InstructionSequence
{
    pub ops: Vec<OpCode>
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

impl InstructionSequence
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

pub struct StackFrame
{
    /// Current VM thread-local stack state
    #[cfg(not(feature="register-vm"))]
    pub stack: Vec<SystemValue>,

    /// Local thread-local variables allocated at this frame
    pub locals: HashMap<VariableIdentifier, RawValue>
}

pub struct VirtualMachine
{
    /// A mapping of global string identifiers to their value
    #[cfg(feature="async")]
    pub globals: Arc<RwLock<HashMap<VariableIdentifier, RawValue>>>,

    /// A mapping of global string identifiers to their value
    #[cfg(not(feature="async"))]
    pub globals: RefCell<HashMap<VariableIdentifier, RawValue>>
}

#[inline(always)]
fn process_address(offset_out: &mut usize, address: &AddressType)
{
    match address {
        AddressType::RelativeOffset { offset } => {
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
        AddressType::AbsoluteTarget { index } => {
            *offset_out = *index;
        }
    }
}

impl VirtualMachine
{
    #[inline(always)]
    #[cfg(feature="async")]
    pub fn new() -> Self {
        let globals: Arc<RwLock<HashMap<VariableIdentifier, RawValue>>> = Arc::new(RwLock::new(HashMap::new()));
        let mut globals_write = globals.write().unwrap();
        globals_write.reserve(1024);
        drop(globals_write);

        return Self {
            globals: globals
        };
    }

    #[inline(always)]
    #[cfg(not(feature="async"))]
    pub fn new() -> Self {
        let mut globals: HashMap<VariableIdentifier, RawValue> = HashMap::new();
        globals.reserve(1024);

        return Self {
            globals: RefCell::new(globals)
        };
    }
    
    pub fn interpret(&self, instructions: &InstructionSequence) -> Result<(), &'static str>
    {
        // Allocate new frame
        let mut stack: Vec<SystemValue> = Vec::new();
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
                    //let current_value = self.stack.last_mut().unwrap();
                    //current_value.negate(self);
                },
                OpCode::Not {  } => {
                    let current_value = frame.stack.pop().unwrap();
                    frame.stack.push(SystemValue::Raw { value: RawValue::Boolean { 0: BooleanValue { value: !current_value.as_raw(self, &frame).as_boolean(self, &frame) }}});
                },
                OpCode::CallFunction {  } => {
                    panic!("Not Implemented");
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
                OpCode::PushLocalReference { variable } => {
                    frame.stack.push(SystemValue::Variable { value: variable.clone() });
                },
                OpCode::PushGlobalReference { variable } => {
                    frame.stack.push(SystemValue::Variable { value: variable.clone() });
                }
            }
        }
    }
}