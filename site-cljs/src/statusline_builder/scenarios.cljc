(ns statusline-builder.scenarios)

(def all
  [{:id "active"
    :label "Active worktree"
    :model "Opus 5 (1M context)"
    :effort "high"
    :context 42
    :cache-ttl-seconds 3600
    :cache-remaining-seconds 2720
    :rate-limits {"FiveHour" {:used 36 :countdown "3.1"}
                  "SevenDay" {:used 21 :countdown "4.8"}}
    :cwd "~/AI.Admin/statusline-rs"
    :git true
    :branch "feature/builder-site"
    :worktree true
    :ahead 2
    :behind 0
    :state ""
    :diff {:modified 2 :untracked 1 :deleted 0}
    :idle-seconds 42
    :cpu "1.10c"
    :rss "685"
    :agents {:active 2 :total 7 :longest-seconds 252 :tokens 1240000 :stalled false}
    :reminders ["standup 11:00"]
    :notice {:text "✗ cargo test (exit 101)" :remaining-seconds 595}
    :last-prompt "rebase the builder branch onto develop and check every segment"
    :llm-answer "Keep the prompt short and let the model ask for what it needs."
    :llm-insight "goal: ship the builder site with the next release; the catalogue is generated"
    :weather "⛅ +21°C"}
   {:id "fable"
    :label "Fable session"
    :model "Fable 5.1"
    :effort "max"
    :context 63
    :cache-ttl-seconds 3600
    :cache-remaining-seconds 1210
    :rate-limits {"FiveHour" {:used 48 :countdown "2.2"}
                  "SevenDay" {:used 31 :countdown "3.9"}
                  "Fable" {:used 76 :countdown "1.5" :severity "warning" :active true}}
    :cwd "~/AI.Admin/statusline-rs"
    :git true
    :branch "develop"
    :worktree false
    :ahead 0
    :behind 0
    :state ""
    :diff {:modified 4 :untracked 0 :deleted 1}
    :idle-seconds 9
    :cpu "0.84c"
    :rss "912"
    :agents {:active 1 :total 2 :longest-seconds 61 :tokens 380000 :stalled false}
    :reminders []
    :notice nil
    :last-prompt "make the 5h window radial"
    :llm-answer "Name the segment after what it shows, not after where it reads."
    :llm-insight "goal: recolour the line; the 5h window is being switched to radial"
    :weather "🌦 +18°C"}
   {:id "pressure"
    :label "Rebase pressure"
    :model "Opus 5"
    :effort "max"
    :context 88
    :cache-ttl-seconds 300
    :cache-remaining-seconds 0
    :rate-limits {"FiveHour" {:used 84 :countdown "0.6"}
                  "SevenDay" {:used 92 :countdown "0.9"}}
    :cwd "~/multicast/escape2"
    :git true
    :branch "fix/input-buffer"
    :worktree false
    :ahead 1
    :behind 3
    :state "REBASE 2/5"
    :diff {:modified 8 :untracked 2 :deleted 1}
    :idle-seconds 367
    :cpu "2.37c"
    :rss "1214"
    :agents {:active 1 :total 3 :longest-seconds 3720 :tokens 0 :stalled true}
    :reminders ["stand up" "drink water"]
    :notice {:text "do not push before the review" :remaining-seconds nil}
    :last-prompt "continue the rebase and fix the conflicts in the input buffer"
    :llm-answer "Resolve the smallest conflict first."
    :llm-insight "goal: finish the rebase; conflicts in the input buffer are being resolved"
    :weather "🌧 +12°C"}
   {:id "clean"
    :label "Clean branch"
    :model "Sonnet 5"
    :effort "medium"
    :context 12
    :cache-ttl-seconds 3600
    :cache-remaining-seconds 484
    :rate-limits {"FiveHour" {:used 12 :countdown "4.4"}
                  "SevenDay" {:used 8 :countdown "6.3"}}
    :cwd "~/courses"
    :git true
    :branch "main"
    :worktree false
    :ahead 0
    :behind 0
    :state ""
    :diff {:modified 0 :untracked 0 :deleted 0}
    :idle-seconds 7
    :cpu "0.06c"
    :rss "224"
    :agents {:active 0 :total 7 :longest-seconds nil :tokens 42000 :stalled false}
    :reminders []
    :notice nil
    :last-prompt "summarise the lecture notes"
    :llm-answer "Ask for an outline before the full text."
    :llm-insight "goal: summarise the notes; an outline was just produced"
    :weather "☀️ +25°C"}
   {:id "outside"
    :label "Outside Git"
    :model "Haiku 4.5"
    :effort "low"
    :context 27
    :cache-ttl-seconds 300
    :cache-remaining-seconds 0
    :rate-limits {"FiveHour" {:used 27 :countdown "2.8"}
                  "SevenDay" {:used 19 :countdown "5.1"}}
    :cwd "~/Downloads"
    :git false
    :branch ""
    :worktree false
    :ahead 0
    :behind 0
    :state ""
    :diff {:modified 0 :untracked 0 :deleted 0}
    :idle-seconds 95
    :cpu "0.22c"
    :rss "312"
    :agents {:active 0 :total 0 :longest-seconds nil :tokens 0 :stalled false}
    :reminders ["take the coffee"]
    :notice nil
    :last-prompt "rename the downloaded files by date"
    :llm-answer "Sort by modification time, then rename."
    :llm-insight "goal: tidy the downloads; files are being renamed by date"
    :weather "🌫 +9°C"}])
