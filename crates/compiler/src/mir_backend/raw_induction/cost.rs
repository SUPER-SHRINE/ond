//! Count successful paths through the actual fixed-up Kagura instructions.
use super::{Plan, mir};
use crate::kagura_encoding::word;
use crate::mir_backend::function::EmittedLayout;
use std::ops::Range;

type Emitted<'a> = (&'a [u32], &'a EmittedLayout);

pub(super) fn improves(
    plan: &Plan,
    f: &mir::Function,
    before: Emitted<'_>,
    after: Emitted<'_>,
    (before_frame, after_frame): (u32, u32),
) -> bool {
    after_frame <= before_frame
        && worst_delta(plan, f, before, after).is_some_and(|delta| delta < 0)
}

fn worst_delta(
    plan: &Plan,
    f: &mir::Function,
    (before, l): Emitted<'_>,
    (after, r): Emitted<'_>,
) -> Option<i128> {
    let flow = &plan.flow;
    let prologue = |words: &[u32], code: &EmittedLayout| {
        steps(
            words,
            0..code.prologue_end,
            code.prologue_end,
            Some(code.blocks[f.entry.0 as usize].start),
            None,
        )
    };
    let site = |words: &[u32],
                code: &EmittedLayout,
                id: mir::BlockId,
                target: Option<mir::BlockId>,
                taken: Option<bool>| {
        let block = &code.blocks[id.0 as usize];
        steps(
            words,
            block.start..block.end,
            block.terminator,
            target.map(|to| code.blocks[to.0 as usize].start),
            taken,
        )
    };
    let edge = |id, target, taken| {
        let prev = site(before, l, id, target, taken)?;
        let next = site(after, r, id, target, taken)?;
        Some(i128::from(next) - i128::from(prev))
    };
    let fixed = i128::from(prologue(after, r)?) - i128::from(prologue(before, l)?)
        + edge(flow.entry, Some(flow.header), None)?
        + edge(flow.header, Some(flow.exit), Some(true))?
        + edge(flow.exit, None, None)?;
    let mut tail = vec![0; f.blocks.len()];
    tail[flow.post.0 as usize] = edge(flow.post, Some(flow.header), None)?;
    for &id in flow.order.iter().rev() {
        tail[id.0 as usize] = match &f.blocks[id.0 as usize].terminator.kind {
            mir::TerminatorKind::Jump(to) => {
                edge(id, Some(to.target), None)? + tail[to.target.0 as usize]
            }
            mir::TerminatorKind::Branch {
                then_edge,
                else_edge,
                ..
            } => {
                let yes = edge(id, Some(then_edge.target), Some(false))?
                    + tail[then_edge.target.0 as usize];
                let no = edge(id, Some(else_edge.target), Some(true))?
                    + tail[else_edge.target.0 as usize];
                yes.max(no)
            }
            _ => return None,
        }
    }
    let iteration = edge(flow.header, Some(flow.first), Some(false))? + tail[flow.first.0 as usize];
    Some(fixed + i128::from(plan.trips) * iteration)
}

/// Count fetches through one block/edge. JZ to a fixed PC-relative address
/// skips branch island padding and symbol literals. A guard's success jumps
/// over the immediately following trap. A MIR branch selects one of the
/// terminator's two routes. Other internal conditionals/register jumps
/// (including far islands) are deliberately unmodeled.
pub(super) fn steps(
    words: &[u32],
    code: Range<usize>,
    terminator: usize,
    destination: Option<usize>,
    mut branch_taken: Option<bool>,
) -> Option<u32> {
    let mut pc = code.start;
    let mut count = 0u32;
    while code.contains(&pc) {
        let inst = *words.get(pc)?;
        count = count.checked_add(1)?;
        let mut next = pc + 1;
        match inst >> 28 {
            12 => {
                let dst = (inst >> 24) & 0xf;
                let cond = (inst >> 20) & 0xf;
                let base = (inst >> 16) & 0xf;
                if base != 0 {
                    return (destination.is_none()
                        && branch_taken.is_none()
                        && next == code.end
                        && inst == word(12, 0, 0, 15, 0))
                    .then_some(count);
                }
                let displacement = inst as i16;
                let take = if cond == 0
                    || (dst == 0
                        && displacement == 1
                        && words.get(next) == Some(&word(6, 0, 0, 0, 0)))
                {
                    true
                } else if pc >= terminator && dst == 0 {
                    branch_taken.take()?
                } else {
                    return None;
                };
                if take {
                    next = next.checked_add_signed(isize::from(displacement))?;
                }
            }
            6 | 14 | 15 => return None,
            _ => {}
        }
        if Some(next) == destination {
            return (branch_taken.is_none()).then_some(count);
        }
        if next <= pc {
            return None;
        }
        pc = next;
    }
    None
}
