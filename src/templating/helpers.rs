use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext, RenderError,
    RenderErrorReason, Renderable,
};
use which::which;

pub struct IfInstalledHelper {
    pub positive: bool,
}

impl HelperDef for IfInstalledHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let bin = h
            .param(0)
            .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("if-installed", 0))?;
        let bin = if bin.is_value_missing() {
            bin.relative_path()
                .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("if-installed", 0))?
                .to_owned()
        } else {
            bin.value().to_string()
        };
        log::debug!("Checking if `{bin}` is installed");

        if which(&bin).is_ok() == self.positive {
            log::debug!("`{bin}` is installed");
            h.template()
                .ok_or_else(|| RenderErrorReason::BlockContentRequired)?
                .render(r, ctx, rc, out)
                .map_err(RenderError::from)
        } else {
            log::debug!("`{bin}` is not installed");
            HelperResult::Ok(())
        }
    }
}
