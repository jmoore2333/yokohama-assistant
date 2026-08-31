// Reading one panel out of a dashboard, in a test.
//
// `FrameModel.dashboard` is a list of externally-tagged variants and `Hand` is
// ABSENT — not empty — for a seat the state does not hold, so every reader has
// to narrow. The client does this inline where it needs it; a test that
// repeated the narrowing per assertion would be four copies of a type guard,
// and a wrong one would read as a missing panel rather than as a bug in the
// test.

import type {
  AchievementModel,
  ClientState,
  HandPanel,
  Panel,
  PlayerModel,
  TracksPanel,
} from "../types";

export const playersOf = (s: ClientState): PlayerModel[] =>
  s.frame.dashboard.find((p): p is Extract<Panel, { Players: unknown }> => "Players" in p)
    ?.Players.players ?? [];

export const tracksOf = (s: ClientState): TracksPanel => {
  const panel = s.frame.dashboard.find(
    (p): p is Extract<Panel, { Tracks: TracksPanel }> => "Tracks" in p,
  );
  if (!panel) throw new Error("this fixture carries no Tracks panel");
  return panel.Tracks;
};

export const achievementsOf = (s: ClientState): AchievementModel[] =>
  s.frame.dashboard.find((p): p is Extract<Panel, { Achievements: unknown }> => "Achievements" in p)
    ?.Achievements.cards ?? [];

/** `undefined` when the payload holds no hand for its seat, which is a real
 *  state and not a failure. */
export const handOf = (s: ClientState): HandPanel | undefined =>
  s.frame.dashboard.find((p): p is Extract<Panel, { Hand: HandPanel }> => "Hand" in p)?.Hand;
