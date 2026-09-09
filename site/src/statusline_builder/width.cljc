(ns statusline-builder.width)

#?(:cljs
   (do
     (def ^:private control-only (js/RegExp. "^[\\u0000-\\u001f\\u007f-\\u009f]*$" "u"))
     (def ^:private pictographic (js/RegExp. "\\p{Extended_Pictographic}" "u"))
     (def ^:private flag (js/RegExp. "^\\p{Regional_Indicator}{2}$" "u"))
     (def ^:private keycap (js/RegExp. "\\u20e3" "u"))
     (def ^:private marks-only (js/RegExp. "^[\\p{Mark}\\p{Format}]+$" "u"))))

(defn- wide-code-point? [cp]
  (and (>= cp 0x1100)
       (or (<= cp 0x115f)
           (= cp 0x2329)
           (= cp 0x232a)
           (and (>= cp 0x2e80) (<= cp 0xa4cf) (not= cp 0x303f))
           (and (>= cp 0xac00) (<= cp 0xd7a3))
           (and (>= cp 0xf900) (<= cp 0xfaff))
           (and (>= cp 0xfe10) (<= cp 0xfe19))
           (and (>= cp 0xfe30) (<= cp 0xfe6f))
           (and (>= cp 0xff00) (<= cp 0xff60))
           (and (>= cp 0xffe0) (<= cp 0xffe6))
           (and (>= cp 0x1b000) (<= cp 0x1b2ff))
           (and (>= cp 0x20000) (<= cp 0x3fffd)))))

(defn graphemes [text]
  #?(:cljs (vec (js/Array.from (.segment (js/Intl.Segmenter. js/undefined #js {:granularity "grapheme"}) text)
                               #(unchecked-get % "segment")))
     :clj (vec (re-seq #"\X" text))))

(defn- grapheme-width [grapheme]
  #?(:cljs (cond
             (.test control-only grapheme) 0
             (.test pictographic grapheme) 2
             (or (.test flag grapheme) (.test keycap grapheme)) 2
             (wide-code-point? (.codePointAt grapheme 0)) 2
             (.test marks-only grapheme) 0
             :else 1)
     :clj (let [cp (.codePointAt ^String grapheme 0)]
            (cond
              (re-matches #"^[\u0000-\u001f\u007f-\u009f]*$" grapheme) 0
              (re-find #"\p{IsExtended_Pictographic}" grapheme) 2
              (re-matches #"^[\x{1f1e6}-\x{1f1ff}]{2}$" grapheme) 2
              (re-find #"\u20e3" grapheme) 2
              (wide-code-point? cp) 2
              (re-matches #"^[\p{M}\p{Cf}]+$" grapheme) 0
              :else 1))))

(defn display-width [text]
  (reduce + 0 (map grapheme-width (graphemes text))))

(defn cut [text max-chars]
  (let [characters #?(:cljs (vec (js/Array.from text)) :clj (vec (re-seq #"(?s)." text)))]
    (if (or (zero? max-chars) (<= (count characters) max-chars))
      text
      (str (apply str (subvec characters 0 (max 0 (dec max-chars)))) "…"))))
