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
    :model-tokens [["opus-5" 3400000] ["haiku-4-5" 45000]]
    :token-spend {"LastTurn" {:input 1200 :output 3400 :thinking 1100 :cache-read 51600 :cache-write 1900}
                  "Session" {:input 12000 :output 210000 :thinking 70000 :cache-read 3043000 :cache-write 180000}}
    :cost 18.4
    :reminders ["standup 11:00"]
    :notice {:text "✗ cargo test (exit 101)" :remaining-seconds 595}
    :last-prompt "rebase the builder branch onto develop and check every segment"
    :command-output "rtt min/avg/max/mdev = 11.842/11.842/11.842/0.000 ms"
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
    :model-tokens [["fable-5-1" 2100000] ["opus-5" 380000]]
    :token-spend {"LastTurn" {:input 800 :output 9100 :thinking 5200 :cache-read 96000 :cache-write 4300}
                  "Session" {:input 9000 :output 140000 :thinking 61000 :cache-read 2201000 :cache-write 130000}}
    :cost 31.75
    :reminders []
    :notice nil
    :last-prompt "make the 5h window radial"
    :command-output "rtt min/avg/max/mdev = 23.517/23.517/23.517/0.000 ms"
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
    :model-tokens [["opus-5" 9800000]]
    :token-spend {"LastTurn" {:input 300 :output 700 :thinking 0 :cache-read 410000 :cache-write 38000}
                  "Session" {:input 20000 :output 380000 :thinking 150000 :cache-read 9000000 :cache-write 400000}}
    :cost 52.1
    :reminders ["stand up" "drink water"]
    :notice {:text "do not push before the review" :remaining-seconds nil}
    :last-prompt "continue the rebase and fix the conflicts in the input buffer"
    :command-output "rtt min/avg/max/mdev = 9.306/9.306/9.306/0.000 ms"
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
    :model-tokens [["sonnet-5" 640000] ["haiku-4-5" 42000]]
    :token-spend {"LastTurn" {:input 90 :output 1500 :thinking 400 :cache-read 22000 :cache-write 600}
                  "Session" {:input 4000 :output 51000 :thinking 12000 :cache-read 580000 :cache-write 47000}}
    :cost 2.35
    :reminders []
    :notice nil
    :last-prompt "summarise the lecture notes"
    :command-output "rtt min/avg/max/mdev = 41.078/41.078/41.078/0.000 ms"
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
    :model-tokens [["haiku-4-5" 12000]]
    :token-spend {"LastTurn" {:input 40 :output 900 :thinking 0 :cache-read 7000 :cache-write 300}
                  "Session" {:input 500 :output 1500 :thinking 0 :cache-read 9000 :cache-write 1000}}
    :cost 0.04
    :reminders ["take the coffee"]
    :notice nil
    :last-prompt "rename the downloaded files by date"
    :command-output "rtt min/avg/max/mdev = 15.229/15.229/15.229/0.000 ms"
    :llm-insight "goal: tidy the downloads; files are being renamed by date"
    :weather "🌫 +9°C"}])
