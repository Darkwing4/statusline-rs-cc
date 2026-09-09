(ns statusline-builder.preview
  (:require [clojure.string :as str]
            [statusline-builder.colour :as colour]
            [statusline-builder.number :as number]
            [statusline-builder.width :as width]))

(defn- piece [text colour]
  {:text text :colour colour})

(defn- text-piece [config text]
  (when (seq text)
    [(piece (str (config "prefix") text) (colour/css (config "color") 50))]))

(defn format-duration [total-seconds]
  (let [seconds (max 0 (number/floor total-seconds))]
    (cond
      (>= seconds 3600) (str (quot seconds 3600) "h" (quot (rem seconds 3600) 60) "m")
      (>= seconds 60) (str (quot seconds 60) "m" (rem seconds 60) "s")
      :else (str seconds "s"))))

(defn format-duration-padded [total-seconds]
  (let [seconds (max 0 (number/floor total-seconds))
        hours (quot seconds 3600)
        minutes (quot (rem seconds 3600) 60)
        remainder (rem seconds 60)]
    (cond
      (pos? hours) (str hours "h" (number/pad2 minutes) "m")
      (pos? minutes) (str minutes "m" (number/pad2 remainder) "s")
      :else (str remainder "s"))))

(defn- format-tokens [tokens]
  (cond
    (< tokens 1000) (str tokens)
    (< tokens 1000000) (let [thousands (/ tokens 1000)]
                         (if (< thousands 10)
                           (str (number/fixed1 thousands) "k")
                           (str (number/round thousands) "k")))
    :else (str (number/fixed1 (/ tokens 1000000)) "M")))

(defn cache-ttl [session]
  (let [ttl-seconds (max 1 (number/floor (:cache-ttl-seconds session)))
        remaining-seconds (number/floor (:cache-remaining-seconds session))]
    (if (<= remaining-seconds 0)
      {:cold true :percentage (:context session) :text "cold"}
      {:cold false
       :percentage (* (/ (- ttl-seconds remaining-seconds) ttl-seconds) 100)
       :text (format-duration-padded remaining-seconds)})))

(defn- model [config session]
  (let [name (reduce (fn [current [from to]] (if (seq from) (str/replace current from to) current))
                     (:model session)
                     (or (config "replacements") []))]
    (when (seq name)
      [(piece (str (config "prefix") name) (colour/css (config "color") 40))])))

(defn- session-notice [config session]
  (when-let [notice (:notice session)]
    (let [text (width/cut (:text notice) (config "max_chars"))]
      (when (seq text)
        (let [remaining (:remaining-seconds notice)
              body (cond-> (str (config "prefix") text)
                     (and (config "show_remaining") (some? remaining)) (str " (" (format-duration-padded remaining) ")"))]
          [(piece body (colour/css (config "color") 50))])))))

(defn- subagent-stats [config session]
  (let [{:keys [active total stalled longest-seconds tokens] :as stats} (:agents session)]
    (when (and stats (pos? total))
      (let [text (cond-> (if (pos? active)
                           (str (config "prefix") active "/" total)
                           (str (config "prefix") total))
                   stalled (str (config "stall_marker"))
                   (and (pos? active) (some? longest-seconds)) (str " " (format-duration-padded longest-seconds))
                   (and (config "show_tokens") (pos? tokens)) (str " " (format-tokens tokens)))
            colour (cond
                     stalled (config "stall_color")
                     (pos? active) (config "active_color")
                     :else (config "color"))]
        [(piece text (colour/css colour 50))]))))

(defn git-branch [config session]
  (when (:git session)
    (let [branch (str (when (and (config "show_worktree") (:worktree session)) "⑂") (:branch session))
          movements (cond-> []
                      (pos? (:ahead session)) (conj (str "↑" (:ahead session)))
                      (pos? (:behind session)) (conj (str "↓" (:behind session))))]
      (cond-> [(piece branch (colour/css (config "color") 35))]
        (and (config "show_state") (seq (:state session)))
        (conj (piece (str " [" (:state session) "]") (colour/css (config "state_color") 90)))

        (and (config "show_ahead_behind") (seq movements))
        (conj (piece (str "(" (str/join " " movements) ")") (colour/css (config "color") 35)))))))

(defn- git-diff [config session separator-colour]
  (when (:git session)
    (let [{:keys [modified untracked deleted]} (:diff session)
          gap (piece " " (colour/css separator-colour 50))
          counts (cond-> []
                   (pos? modified) (conj (piece (str "~" modified) (colour/css (config "modified_color") 55)))
                   (pos? untracked) (conj (piece (str "+" untracked) (colour/css (config "untracked_color") 35)))
                   (pos? deleted) (conj (piece (str "-" deleted) (colour/css (config "deleted_color") 90))))]
      (when (seq counts)
        (vec (interpose gap counts))))))

(defn- bar-glyph [percentage]
  (nth ["▁" "▂" "▃" "▄" "▅" "▆" "▇" "█"] (max 0 (min 7 (number/floor (/ (* percentage 8) 100))))))

(defn- radial-glyph [percentage]
  (nth ["○" "◔" "◑" "◕" "●"] (max 0 (min 4 (number/floor (/ (* percentage 5) 100))))))

(defn- rate-limit-markers [config data]
  (let [severity (some (fn [[name marker]] (when (and (seq name) (= name (:severity data))) marker))
                       (or (config "severity_markers") []))
        active (when (:active data) (or (config "active_marker") ""))]
    (str severity active)))

(defn- rate-limit [config session]
  (when-let [data (get (:rate-limits session) (config "window"))]
    (let [used (max 0 (min 100 (:used data)))
          glyph-value (if (= "Remaining" (config "fill")) (- 100 used) used)
          prefix (str/replace (config "prefix") "{t}" (:countdown data))
          rounded (number/round used)
          body (case (config "style")
                 "Bar" (str prefix (bar-glyph glyph-value))
                 "BarPercent" (str prefix (bar-glyph glyph-value) " " rounded "%")
                 "Radial" (str prefix (radial-glyph glyph-value))
                 "RadialPercent" (str prefix (radial-glyph glyph-value) " " rounded "%")
                 (str prefix rounded "%"))
          colour (if (= "Steps" (config "color_mode"))
                   (cond
                     (< used 50) (colour/css (config "low_color") used)
                     (<= used 80) (colour/css (config "mid_color") used)
                     :else (colour/css (config "high_color") used))
                   (colour/interpolate-stops (colour/colour->rgb (config "low_color") [166 227 161])
                                             (colour/colour->rgb (config "mid_color") [249 226 175])
                                             (colour/colour->rgb (config "high_color") [243 139 168])
                                             used
                                             (config "gradient_midpoint_percentage")))]
      [(piece (str body (rate-limit-markers config data)) colour)])))

(defn pieces
  ([segment session]
   (pieces segment session (colour/grey)))
  ([segment session separator-colour]
   (let [config (:config segment)]
     (case (:type segment)
       "Model" (model config session)
       "Effort" [(piece (str (config "prefix") (:effort session)) (colour/css (config "color") 45))]
       "ContextUsage" [(piece (config "prefix") (colour/css (config "prefix_color") (:context session)))
                       (piece (str (number/round (:context session)) "%") (colour/context-css (config "color") (:context session)))
                       (piece (config "suffix") (colour/css (config "suffix_color") (:context session)))]
       "PromptCacheTtl" (let [view (cache-ttl session)]
                          [(piece (str (config "prefix") (:text view)) (colour/cache-ttl-css (config "color") view))])
       "ClaudeResourceUsage" [(piece (str (config "cpu_prefix") (:cpu session) " " (config "memory_prefix") (:rss session) " MiB")
                                     (colour/css (config "color") 46))]
       "Cwd" [(piece (:cwd session) (colour/css (config "color") 45))]
       "GitBranch" (git-branch config session)
       "GitDiff" (git-diff config session separator-colour)
       "GitError" (when-not (:git session) [(piece (config "text") (colour/css (config "color") 90))])
       "UserIdleTime" (when (>= (:idle-seconds session) (config "threshold_seconds"))
                        [(piece (str (config "prefix") (format-duration (:idle-seconds session))) (colour/css (config "color") 48))])
       "RateLimit" (rate-limit config session)
       "SubagentStats" (subagent-stats config session)
       "Reminder" (text-piece config (width/cut (str/join (config "separator") (:reminders session)) (config "max_chars")))
       "SessionNotice" (session-notice config session)
       "MyLastPrompt" (text-piece config (width/cut (:last-prompt session) (config "max_chars")))
       "CommandOutput" (text-piece config (width/cut (:command-output session) (config "max_chars")))
       "LlmInsight" (text-piece config (width/cut (:llm-insight session) (config "max_chars")))
       "Weather" (text-piece config (width/cut (:weather session) (config "max_chars")))
       "Spacer" [(piece "\u2060" "transparent")]
       nil))))
