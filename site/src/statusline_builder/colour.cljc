(ns statusline-builder.colour
  (:require [clojure.string :as str]
            [statusline-builder.number :as number]))

(def ansi
  [[30 "Black" "#303642"]
   [31 "Red" "#d85c68"]
   [32 "Green" "#69c88d"]
   [33 "Yellow" "#d4b967"]
   [34 "Blue" "#668fd4"]
   [35 "Magenta" "#ad7ad0"]
   [36 "Cyan" "#58c4d1"]
   [37 "White" "#c9d1dc"]
   [90 "Bright black" "#66758a"]
   [91 "Bright red" "#ff7c88"]
   [92 "Bright green" "#78d49b"]
   [93 "Bright yellow" "#e4c879"]
   [94 "Bright blue" "#7fa8ef"]
   [95 "Bright magenta" "#c99af2"]
   [96 "Bright cyan" "#63d9e6"]
   [97 "Bright white" "#e8edf5"]])

(def context-gradient-stops
  [[0 [150 150 150]]
   [20 [180 165 100]]
   [30 [220 60 60]]])

(def cache-ttl-gradient-stops
  [[0 [120 120 120]]
   [70 [200 180 80]]
   [90 [230 100 60]]
   [100 [255 50 50]]])

(def cache-cold-gradient-stops
  [[0 [120 120 120]]
   [40 [200 180 80]]
   [75 [230 100 60]]
   [100 [255 50 50]]])

(defn rgb-array->hex [channels]
  (str "#" (apply str (map #(number/hex2 (max 0 (min 255 %))) channels))))

(defn hex->rgb [hex]
  (let [digits (str/replace hex "#" "")]
    (mapv #(number/parse-hex (subs digits % (+ % 2))) [0 2 4])))

(defn named [code]
  {:kind :named :code code})

(defn rgb [r g b]
  {:kind :rgb :hex (rgb-array->hex [r g b])})

(def gradient {:kind :gradient})

(defn grey []
  (rgb 120 125 140))

(defn- between [[start-position start-colour] [end-position end-colour] percentage quantize]
  (let [amount (max 0 (min 1 (/ (- percentage start-position) (- end-position start-position))))]
    (mapv (fn [channel end-channel]
            (quantize (max 0 (min 255 (+ channel (* (- end-channel channel) amount))))))
          start-colour
          end-colour)))

(defn gradient-rgb [stops percentage quantization]
  (let [quantize (if (= :truncate quantization) number/trunc number/round)
        [first-position first-colour] (first stops)]
    (cond
      (<= percentage first-position) (vec first-colour)
      :else (or (some (fn [[start end]]
                        (when (<= percentage (first end))
                          (between start end percentage quantize)))
                      (partition 2 1 stops))
                (vec (second (last stops)))))))

(defn interpolate-stops [low middle high percentage midpoint]
  (rgb-array->hex (gradient-rgb [[0 low] [midpoint middle] [100 high]] percentage :nearest)))

(defn css [colour percentage]
  (case (:kind colour)
    :rgb (:hex colour)
    :named (or (some (fn [[code _ hex]] (when (= code (:code colour)) hex)) ansi) "#a6b1c2")
    (interpolate-stops [60 200 60] [220 200 40] [220 60 60] percentage 50)))

(defn colour->rgb [colour fallback]
  (if (= :rgb (:kind colour))
    (hex->rgb (:hex colour))
    fallback))

(defn context-css [colour percentage]
  (if (= :gradient (:kind colour))
    (rgb-array->hex (gradient-rgb context-gradient-stops percentage :truncate))
    (css colour percentage)))

(defn cache-ttl-css [colour {:keys [cold percentage]}]
  (if (= :gradient (:kind colour))
    (rgb-array->hex (gradient-rgb (if cold cache-cold-gradient-stops cache-ttl-gradient-stops) percentage :truncate))
    (css colour percentage)))

(defn swatch [colour]
  (if (= :gradient (:kind colour))
    "linear-gradient(90deg, #78d49b, #e4c879, #ff7c88)"
    (css colour 50)))
