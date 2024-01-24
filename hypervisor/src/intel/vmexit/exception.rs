use {
    crate::{
        intel::{
            ept::hooks::HookType,
            events::EventInjection,
            support::{vmread, vmwrite},
            vmerror::{
                EptViolationExitQualification, ExceptionInterrupt, VmExitInterruptionInformation,
            },
            vmexit::ExitType,
            vmx::Vmx,
        },
        utils::capture::GuestRegisters,
    },
    x86::vmx::vmcs,
};

#[rustfmt::skip]
pub fn handle_exception(_guest_registers: &mut GuestRegisters, vmx: &mut Vmx) -> ExitType {
    log::debug!("Handling ExceptionOrNmi VM exit...");

    let interruption_info_value = vmread(vmcs::ro::VMEXIT_INTERRUPTION_INFO);
    let interruption_error_code_value = vmread(vmcs::ro::VMEXIT_INTERRUPTION_ERR_CODE);

    if let Some(interruption_info) = VmExitInterruptionInformation::from_u32(interruption_info_value as u32) {
        if let Some(exception_interrupt) = ExceptionInterrupt::from_u32(interruption_info.vector.into()) {
            match exception_interrupt {
                ExceptionInterrupt::PageFault => {
                    let exit_qualification_value = vmread(vmcs::ro::EXIT_QUALIFICATION);
                    let ept_violation_qualification = EptViolationExitQualification::from_exit_qualification(exit_qualification_value);
                    log::trace!("Exit Qualification for EPT Violations: {}", ept_violation_qualification);
                    EventInjection::vmentry_inject_pf(interruption_error_code_value as u32);
                },
                ExceptionInterrupt::GeneralProtectionFault => {
                    EventInjection::vmentry_inject_gp(interruption_error_code_value as u32);
                },
                ExceptionInterrupt::Breakpoint => {
                    handle_breakpoint_exception(_guest_registers, vmx);
                },
                _ => {
                    panic!("Unhandled exception: {:?}", exception_interrupt);
                }
            }
        } else {
            panic!("Invalid Exception Interrupt Vector: {}", interruption_info.vector);
        }
    } else {
        panic!("Invalid VM Exit Interruption Information");
    }

    log::debug!("Exception Handled successfully!");

    ExitType::Continue
}

fn handle_breakpoint_exception(guest_registers: &mut GuestRegisters, _vmx: &mut Vmx) {
    log::debug!("Breakpoint Exception");

    let hook_manager = unsafe { _vmx.shared_data.as_mut().hook_manager.as_mut() };

    log::trace!("Finding hook for RIP: {:#x}", guest_registers.rip);

    // Find the handler address for the current instruction pointer (RIP) and
    // transfer the execution to it. If we couldn't find a hook, we inject the
    // #BP exception.
    //
    if let Some(Some(handler)) =
        hook_manager
            .find_hook_by_address(guest_registers.rip)
            .map(|hook| {
                log::trace!("Found hook for RIP: {:#x}", guest_registers.rip);
                if let HookType::Function { inline_hook } = &hook.hook_type {
                    log::trace!("Getting handler address");
                    Some(inline_hook.handler_address())
                } else {
                    None
                }
            })
    {
        // Call our hook handle function (it will automatically call trampoline).
        log::trace!("Transferring execution to handler: {:#x}", handler);
        guest_registers.rip = handler;
        vmwrite(vmcs::guest::RIP, guest_registers.rip);

        log::debug!("Breakpoint (int3) hook handled successfully!");

        ExitType::Continue
    } else {
        EventInjection::vmentry_inject_bp();

        log::debug!("Breakpoint exception handled successfully!");
        ExitType::Continue
    };
}