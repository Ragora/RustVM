use std::{sync, collections::HashMap, time::Duration};

// Import libs
use PerfTest::{vm::{VirtualMachine, InstructionSequence, OpCode, Function, VariableReference, PushFloat, AddressValue, RawValue, VariableIdentifier, SystemValue, StackFrame}, util::variable_name_to_identifier};


use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[derive(Clone)]
struct ApplicationState
{
    pub running: bool
}

fn criterion_benchmark(criterion: &mut Criterion) {
    // Initial Setup
    let call_function_ops = InstructionSequence { 
        ops: vec![
            // FIXME: Encode this ahead of time to avoid the CPU
            OpCode::CallFunction { target: vec!["quit".to_owned()] },
        ]
    };

    let string_append_ops = InstructionSequence { 
        ops: vec![
            // %append = "ABC"
            OpCode::PushString { value: "ABC".to_owned() },
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("append".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // %counter = 0
            OpCode::PushInteger { value: 0 },
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("counter".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // %iterations = 4096
            OpCode::PushInteger { value: 4096 },
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("iterations".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // %result = ""
            OpCode::PushString { value: "".to_owned() },
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("result".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // 12th index is start of program
            OpCode::NOP { },

            // Loop %iterations iterations with current VM state and perform a calculation
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("result".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("append".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::Concat { },
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("result".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::Swap { }, // Crappy workaround until I feel like fixing the stack arrangement
            OpCode::Assignment { },
            OpCode::Pop { },            

            // Increment counter
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("counter".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::PushInteger { value: 1 },
            OpCode::Add { },
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("counter".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // Check if loop condition is met - %counter >= %iterations
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("iterations".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("counter".to_owned()), phantom: std::marker::PhantomData } },

            OpCode::GreaterThanOrEqual { },
            OpCode::JumpFalse { target: AddressValue::AbsoluteTarget { index: 12 } },

            // Write final result to a global
            OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("result".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::PushVariable { variable: VariableReference::Global {value:variable_name_to_identifier("result".to_owned()), phantom: std::marker::PhantomData } },
            OpCode::Assignment {  }
        ]
    };

    // FIXME: Get a more accurate compile result by sourcing this post-compile
    let large_loop_ops = InstructionSequence { 
        ops: vec![
        // Assign %counter = 0
        OpCode::PushInteger { value: 0 },
        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("counter_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::Assignment { },
        OpCode::Pop { },
        
        // Assign %result = 0.0
        OpCode::PushFloat { 0: PushFloat { value: 0.0 }},
        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("result_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::Assignment { },
        OpCode::Pop { },

        // Assign %iterations
        OpCode::PushInteger { value: 4096 },
        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("iterations_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::Assignment { },
        OpCode::Pop { },

        // 12th index is start of program
        OpCode::NOP { },

        // Loop %iterations iterations with current VM state and perform a calculation
        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("result_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::PushFloat { 0: PushFloat { value: 3.14 }},
        OpCode::Add { },

        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("result_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::Swap { }, // Crappy workaround until I feel like fixing the stack arrangement
        OpCode::Assignment { },
        OpCode::Pop { },

        // Increment counter
        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("counter_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::PushInteger { value: 1 },
        OpCode::Add { },
        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("counter_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::Assignment { },
        OpCode::Pop { },

        // Check if loop condition is met - %counter >= %iterations
        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("iterations_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("counter_a".to_owned()), phantom: std::marker::PhantomData } },

        OpCode::GreaterThanOrEqual { },
        OpCode::JumpFalse { target: AddressValue::AbsoluteTarget { index: 12 } },

        // Write final result to a global
        OpCode::PushVariable { variable: VariableReference::Local {value:variable_name_to_identifier("result_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::PushVariable { variable: VariableReference::Global {value:variable_name_to_identifier("result_a".to_owned()), phantom: std::marker::PhantomData } },
        OpCode::Assignment {  }
    ]};

    let vm = VirtualMachine::new(ApplicationState { running: true });

    // Add a native binding for calls
    let mut namespace_write = vm.root_namespace.borrow_mut();
    namespace_write.add_function_entry(Function::NativeFunction { 
        parameters: Vec::new(), 
        binding: Box::new(|_vm, _frame| -> Result<(), &'static str> {
            Ok(())
        })
    }, &vec!["quit".to_owned()]).unwrap();
    drop(namespace_write);
    
    // Ask criterion to execute the tests
    criterion.bench_function("zero parameter calls", |b| b.iter(|| {
        // Use black_box to try and ensure that the entire VM system is ran
        black_box(vm.interpret(&call_function_ops).unwrap());
    }));

    criterion.bench_function("string append - 4096 iterations", |b| b.iter(|| {
        // Use black_box to try and ensure that the entire VM system is ran
        black_box(vm.interpret(&string_append_ops).unwrap());
    }));

    criterion.bench_function("large loop calculation - 4096 iterations", |b| b.iter(|| {
        // Use black_box to try and ensure that the entire VM system is ran
        black_box(vm.interpret(&large_loop_ops).unwrap());

        let globals_read = vm.globals.borrow();
        let result_value = globals_read.get(&variable_name_to_identifier("result_a".to_owned())).unwrap();
        
        // FIXME: API Issue here
        let mut stack: Vec<SystemValue<ApplicationState>> = Vec::new();
        stack.reserve(1024);
        let mut frame = StackFrame {
            locals: HashMap::new(),
            stack: stack
        };
        
        let raw_float = result_value.as_float(&vm, &frame);
    }));
}

criterion_group!{
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs_f64(20.0));
    targets = criterion_benchmark
}

criterion_main!(benches);