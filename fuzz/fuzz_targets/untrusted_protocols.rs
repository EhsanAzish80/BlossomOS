#![no_main]

use blossom_core::{
    ApprovalCapacity, GatewayFrameDecoder, GatewayProfile, ModelProfile, ModelProviderKind,
    RequestOrigin, RequestResolver, SessionContext, ToolRequestWire, decode_gateway_cancel,
    decode_gateway_event, decode_gateway_hello, decode_gateway_private_request,
    decode_gateway_synthetic_request, decode_shell_client_request, WorkspacePlanOrigin,
    WorkspacePlanProposalResolver,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_shell_client_request(data);
    let _ = WorkspacePlanProposalResolver::resolve(
        data,
        WorkspacePlanOrigin::ModelProposedScheme,
    );
    let mut decoder = GatewayFrameDecoder::default();
    if let Ok(frames) = decoder.push(data) {
        for frame in frames {
            let _ = decode_gateway_hello(&frame, GatewayProfile::LlamaCppCpuV1);
            let _ = decode_gateway_synthetic_request(&frame, GatewayProfile::LlamaCppCpuV1);
            if let Ok(model) = ModelProfile::parse("fixture-model:1".into()) {
                let _ = decode_gateway_private_request(&frame, ModelProviderKind::LlamaCpp, model);
            }
            let _ = decode_gateway_cancel(&frame);
            let _ = decode_gateway_event(&frame);
        }
    }
    let _ = decoder.finish();
    if let Ok(input) = std::str::from_utf8(data)
        && let Ok(wire) = ToolRequestWire::parse_json(input)
        && let Ok(capacity) = ApprovalCapacity::new(1)
        && let Ok(reservation) = capacity.reserve()
    {
        let _ = RequestResolver::resolve(
            wire,
            SessionContext {
                workspace_root: "/tmp",
                origin: RequestOrigin::ModelProposed,
            },
            reservation,
        );
    }
});
