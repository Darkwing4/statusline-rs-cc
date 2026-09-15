(ns statusline-builder.number)

(defn round [value]
  #?(:cljs (js/Math.round value)
     :clj (Math/round (double value))))

(defn trunc [value]
  #?(:cljs (js/Math.trunc value)
     :clj (long value)))

(defn floor [value]
  #?(:cljs (js/Math.floor value)
     :clj (long (Math/floor (double value)))))

(defn integer-valued? [value]
  #?(:cljs (js/Number.isInteger value)
     :clj (== value (long value))))

(defn fixed1 [value]
  #?(:cljs (.toFixed value 1)
     :clj (format "%.1f" (double value))))

(defn fixed2 [value]
  #?(:cljs (.toFixed value 2)
     :clj (format "%.2f" (double value))))

(defn pad2 [value]
  #?(:cljs (.padStart (str value) 2 "0")
     :clj (format "%02d" (long value))))

(defn hex2 [value]
  #?(:cljs (.padStart (.toString value 16) 2 "0")
     :clj (format "%02x" (long value))))

(defn parse-hex [text]
  #?(:cljs (js/parseInt text 16)
     :clj (Integer/parseInt text 16)))
