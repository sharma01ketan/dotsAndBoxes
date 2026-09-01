//! Thin WASM bindings for in-browser AlphaZero inference (tract).
//!
//! Base `@dab/dab-wasm` stays tract-free. This module is lazily fetched.
//! PUCT is [`dab_core::AzEngine`]; HUD wiring is slice E.
//! See `docs/specs/phase4-in-wasm-az.md`.

use std::cell::RefCell;

use dab_core::{
    AzEngine, BoardGeom, EdgeId, Engine, Evaluate, Game, AZ_CHANNELS, AZ_FEATURES, AZ_PLANE,
    AZ_POLICY,
};
use tract_onnx::prelude::*;
use wasm_bindgen::prelude::*;

#[cfg(test)]
mod fixture;
mod stamp;

use stamp::{compiled_source_stamp, validate_sidecar, StampError};

#[wasm_bindgen(js_name = AZ_CHANNELS)]
pub fn az_channels() -> u32 {
    AZ_CHANNELS as u32
}

#[wasm_bindgen(js_name = AZ_PLANE)]
pub fn az_plane() -> u32 {
    AZ_PLANE as u32
}

#[wasm_bindgen(js_name = AZ_POLICY)]
pub fn az_policy() -> u32 {
    AZ_POLICY as u32
}

struct Loaded {
    sidecar_json: String,
    eval: TractEval,
}

thread_local! {
    static MODEL: RefCell<Option<Loaded>> = const { RefCell::new(None) };
}

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

fn js_err(msg: impl Into<String>) -> JsValue {
    JsValue::from_str(&msg.into())
}

fn stamp_err(err: StampError) -> JsValue {
    js_err(err.as_str())
}

/// Parse ONNX via tract, validate the sidecar stamp, store the model.
///
/// Throws on any stamp mismatch. Slice A: a fixture net is enough; HUD is slice E.
#[wasm_bindgen(js_name = loadAzModel)]
pub fn load_az_model(onnx: &[u8], sidecar: &str) -> Result<(), JsValue> {
    let _parsed = validate_sidecar(sidecar, onnx, compiled_source_stamp()).map_err(stamp_err)?;
    let eval = load_tract_inner(onnx).map_err(js_err)?;
    MODEL.with(|slot| {
        *slot.borrow_mut() = Some(Loaded {
            sidecar_json: sidecar.to_string(),
            eval,
        });
    });
    Ok(())
}

/// Loaded sidecar JSON, or `""` if none.
#[wasm_bindgen(js_name = azModelStamp)]
pub fn az_model_stamp() -> String {
    MODEL.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|m| m.sidecar_json.clone())
            .unwrap_or_default()
    })
}

/// Mirror of the Worker's `WasmGame`. Own state; does not share the base module.
#[wasm_bindgen]
pub struct AzGame {
    inner: Game,
}

#[wasm_bindgen]
impl AzGame {
    #[wasm_bindgen(constructor)]
    pub fn new(rows: u8, cols: u8) -> Result<AzGame, JsValue> {
        let geom = BoardGeom::new(rows, cols)
            .ok_or_else(|| js_err(format!("invalid board size {rows}x{cols}")))?;
        Ok(Self {
            inner: Game::new(geom),
        })
    }

    /// Keep the mirror in sync. Does not search.
    pub fn play(&mut self, edge: u16) -> Result<(), JsValue> {
        self.inner
            .play(edge)
            .map_err(|_| js_err(format!("illegal edge {edge}")))?;
        Ok(())
    }

    /// Net policy argmax over legal moves. Requires a loaded model. No tree search.
    #[wasm_bindgen(js_name = policyArgmax)]
    pub fn policy_argmax(&self, last_move: i32) -> Result<u16, JsValue> {
        if self.inner.is_terminal() {
            return Err(js_err("cannot choose a move on a terminal game"));
        }
        let last = decode_last_move(last_move);
        MODEL.with(|slot| {
            let loaded = slot.borrow();
            let loaded = loaded
                .as_ref()
                .ok_or_else(|| js_err("AZ model is not loaded"))?;
            let engine = AzEngine::new(&loaded.eval, 0).with_last_move(last);
            Ok(engine.policy_argmax(&self.inner))
        })
    }

    /// PUCT search. Requires a loaded model. Does not apply the chosen edge.
    /// Endgame Perfect/CGT handoff is slice C.
    #[wasm_bindgen(js_name = chooseMoveAz)]
    pub fn choose_move_az(&self, last_move: i32, sims: u32, seed: u64) -> Result<u16, JsValue> {
        if self.inner.is_terminal() {
            return Err(js_err("cannot choose a move on a terminal game"));
        }
        let last = decode_last_move(last_move);
        MODEL.with(|slot| {
            let loaded = slot.borrow();
            let loaded = loaded
                .as_ref()
                .ok_or_else(|| js_err("AZ model is not loaded"))?;
            let mut engine = AzEngine::new(&loaded.eval, seed)
                .with_sims(sims)
                .with_last_move(last);
            Ok(engine.choose(&self.inner))
        })
    }
}

fn decode_last_move(last_move: i32) -> Option<EdgeId> {
    if last_move < 0 {
        None
    } else {
        Some(last_move as EdgeId)
    }
}

struct TractEval {
    model: TypedRunnableModel<TypedModel>,
}

impl Evaluate for TractEval {
    fn evaluate(&self, features: &[f32]) -> (Vec<f32>, f32) {
        debug_assert_eq!(features.len(), AZ_FEATURES);
        let input = Tensor::from_shape(&[1, AZ_CHANNELS, AZ_PLANE, AZ_PLANE], features)
            .expect("feature shape");
        let result = self.model.run(tvec!(input.into())).expect("tract forward");
        let policy = result[0]
            .to_array_view::<f32>()
            .expect("policy f32")
            .iter()
            .copied()
            .collect::<Vec<_>>();
        let value = result[1]
            .to_array_view::<f32>()
            .expect("value f32")
            .iter()
            .copied()
            .next()
            .expect("scalar value");
        (policy, value.clamp(-1.0, 1.0))
    }
}

fn load_tract_inner(onnx: &[u8]) -> Result<TractEval, String> {
    let model = tract_onnx::onnx()
        .model_for_read(&mut std::io::Cursor::new(onnx))
        .map_err(|e| format!("tract parse: {e}"))?
        .into_optimized()
        .map_err(|e| format!("tract optimize: {e}"))?
        .into_runnable()
        .map_err(|e| format!("tract runnable: {e}"))?;
    Ok(TractEval { model })
}

#[cfg(test)]
mod tests {
    use super::stamp::{sha256_hex, validate_sidecar};
    use dab_core::{Evaluate, AZ_CHANNELS, AZ_FEATURES, AZ_PLANE, AZ_POLICY};

    #[test]
    fn consts_match_core() {
        assert_eq!(AZ_CHANNELS, 7);
        assert_eq!(AZ_PLANE, 11);
        assert_eq!(AZ_POLICY, 60);
        assert_eq!(AZ_FEATURES, 847);
    }

    #[test]
    fn stamp_accepts_dev_fixture_bytes() {
        let onnx = b"fixture";
        let sha = sha256_hex(onnx);
        let ok = format!(
            r#"{{"schema":"dab-az-model/1","name":"az-5x5-v1","boardRows":5,"boardCols":5,"channels":7,"plane":11,"policyLength":60,"featuresVersion":1,"sourceStamp":"dev","onnxSha256":"{sha}"}}"#
        );
        assert!(validate_sidecar(&ok, onnx, "dev").is_ok());
    }

    #[test]
    fn fixture_onnx_policy_len_60_value_in_range() {
        let bytes = crate::fixture::dummy_onnx();
        let eval = crate::load_tract_inner(&bytes).expect("tract load fixture");
        let feat = [0.0f32; AZ_FEATURES];
        let (policy, value) = eval.evaluate(&feat);
        assert_eq!(policy.len(), AZ_POLICY);
        assert!(
            (-1.0..=1.0).contains(&value),
            "value {value} outside [-1, 1]"
        );
        assert!(policy.iter().all(|p| *p == 0.0));
        assert_eq!(value, 0.0);
    }
}
