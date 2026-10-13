                // Temporary Voxy diagnostic: isolate contraction/relaxed arithmetic.
                // Default Metal compilation remains unchanged.
                let diagnostic_math=std::env::var("VOXY_DIAGNOSTIC_METAL_SAFE_MATH").ok();
                let diagnostic_safe=match diagnostic_math.as_deref() {
                    Some("compute") => stage_bit == wgt::ShaderStages::COMPUTE,
                    Some("render") => stage_bit != wgt::ShaderStages::COMPUTE,
                    Some(_) => true,
                    None => false,
                };
                if diagnostic_safe {
                    #[allow(deprecated)]
                    options.setFastMathEnabled(false);
                }


