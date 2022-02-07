// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::{interop::SemanticModel, qir};
use inkwell::attributes::AttributeLoc;
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;
use qirlib::codegen::ext::{BasicValues, Qubits, Types};
use qirlib::passes::run_basic_passes_on;
use qirlib::{codegen::CodeGenerator, module};
use std::collections::HashMap;

/// # Errors
///
/// Will return `Err` if module fails verification that the current `Module` is valid.
pub fn write_model_to_file(model: &SemanticModel, file_name: &str) -> Result<(), String> {
    let ctx = inkwell::context::Context::create();
    let generator = populate_context(&ctx, model)?;
    run_basic_passes_on(&generator.module);
    generator.emit_ir(file_name)?;

    Ok(())
}

/// # Errors
///
/// Will return `Err` if module fails verification that the current `Module` is valid.
pub fn get_ir_string(model: &SemanticModel) -> Result<String, String> {
    let ctx = inkwell::context::Context::create();
    let generator = populate_context(&ctx, model)?;
    run_basic_passes_on(&generator.module);
    let ir = generator.get_ir_string();

    Ok(ir)
}

/// # Errors
///
/// Will return `Err` if module fails verification that the current `Module` is valid.
pub fn get_bitcode_base64_string(model: &SemanticModel) -> Result<String, String> {
    let ctx = inkwell::context::Context::create();
    let generator = populate_context(&ctx, model)?;
    run_basic_passes_on(&generator.module);

    let b64 = generator.get_bitcode_base64_string();

    Ok(b64)
}

/// # Errors
///
/// Will return `Err` if
///  - module cannot be loaded.
///  - module fails verification that the current `Module` is valid.
pub fn populate_context<'a>(
    ctx: &'a inkwell::context::Context,
    model: &'a SemanticModel,
) -> Result<CodeGenerator<'a>, String> {
    let module = module::load_template(&model.name, ctx)?;
    let generator = CodeGenerator::new(ctx, module)?;
    build_entry_function(&generator, model)?;
    Ok(generator)
}

fn build_entry_function(
    generator: &CodeGenerator<'_>,
    model: &SemanticModel,
) -> Result<(), String> {
    let entrypoint = qir::create_entrypoint_function(generator.context, &generator.module)?;

    if model.static_alloc {
        let num_qubits = format!("{}", model.qubits.len());
        let required_qubits = generator
            .context
            .create_string_attribute("requiredQubits", &num_qubits);
        entrypoint.add_attribute(AttributeLoc::Function, required_qubits);
    }

    let entry = generator.context.append_basic_block(entrypoint, "entry");
    generator.builder.position_at_end(entry);

    let qubits = write_qubits(model, generator);

    let mut registers = write_registers(model);

    write_instructions(model, generator, &qubits, &mut registers);

    if !model.static_alloc {
        free_qubits(generator, &qubits);
    }

    generator.builder.build_return(None);

    generator.module.verify().map_err(|e| e.to_string())
}

fn free_qubits<'ctx>(
    generator: &CodeGenerator<'ctx>,
    qubits: &HashMap<String, BasicValueEnum<'ctx>>,
) {
    for (_, value) in qubits.iter() {
        generator.emit_release_qubit(value);
    }
}

fn write_qubits<'ctx>(
    model: &SemanticModel,
    generator: &CodeGenerator<'ctx>,
) -> HashMap<String, BasicValueEnum<'ctx>> {
    if model.static_alloc {
        let mut qubits: HashMap<String, BasicValueEnum<'ctx>> = HashMap::new();
        for (id, qubit) in model.qubits.iter().enumerate() {
            let indexed_name = format!("{}{}", &qubit.name[..], qubit.index);
            let int_value = generator.usize_to_i64(id).into_int_value();
            let qubit_ptr_type = generator.qubit_type().ptr_type(AddressSpace::Generic);

            let intptr =
                generator
                    .builder
                    .build_int_to_ptr(int_value, qubit_ptr_type, &indexed_name);
            qubits.insert(indexed_name, intptr.into());
        }
        qubits
    } else {
        let qubits = model
            .qubits
            .iter()
            .map(|reg| {
                let indexed_name = format!("{}{}", &reg.name[..], reg.index);
                let value = generator.emit_allocate_qubit(indexed_name.as_str());
                (indexed_name, value)
            })
            .collect();

        qubits
    }
}

fn write_registers<'ctx>(model: &SemanticModel) -> HashMap<String, Option<PointerValue<'ctx>>> {
    let mut registers = HashMap::new();
    let number_of_registers = model.registers.len() as u64;
    if number_of_registers > 0 {
        for register in &model.registers {
            for index in 0..register.size {
                let name = format!("{}{}", register.name, index);
                registers.insert(name, None);
            }
        }
    }
    registers
}

fn write_instructions<'ctx>(
    model: &SemanticModel,
    generator: &CodeGenerator<'ctx>,
    qubits: &HashMap<String, BasicValueEnum<'ctx>>,
    registers: &mut HashMap<String, Option<PointerValue<'ctx>>>,
) {
    for inst in &model.instructions {
        qir::instructions::emit(generator, inst, qubits, registers);
    }
}
