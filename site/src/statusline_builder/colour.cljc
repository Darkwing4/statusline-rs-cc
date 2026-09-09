(ns statusline-builder.colour
  (:require [clojure.string :as str]
            [statusline-builder.number :as number]))

(def palette
  [["Rosewater" "#f5e0dc"]
   ["Flamingo" "#f2cdcd"]
   ["Pink" "#f5c2e7"]
   ["Mauve" "#cba6f7"]
   ["Red" "#f38ba8"]
   ["Maroon" "#eba0ac"]
   ["Peach" "#fab387"]
   ["Yellow" "#f9e2af"]
   ["Green" "#a6e3a1"]
   ["Teal" "#94e2d5"]
   ["Sky" "#89dceb"]
   ["Sapphire" "#74c7ec"]
   ["Blue" "#89b4fa"]
   ["Lavender" "#b4befe"]
   ["Text" "#cdd6f4"]
   ["Subtext 1" "#bac2de"]
   ["Subtext 0" "#a6adc8"]
   ["Overlay 2" "#9399b2"]
   ["Overlay 1" "#7f849c"]
   ["Overlay 0" "#6c7086"]])

(def context-gradient-stops
  [[0 [147 153 178]]
   [20 [249 226 175]]
   [30 [243 139 168]]])

(def cache-ttl-gradient-stops
  [[0 [147 153 178]]
   [70 [249 226 175]]
   [90 [250 179 135]]
   [100 [243 139 168]]])

(def cache-cold-gradient-stops
  [[0 [147 153 178]]
   [40 [249 226 175]]
   [75 [250 179 135]]
   [100 [243 139 168]]])

(defn rgb-array->hex [channels]
  (str "#" (apply str (map #(number/hex2 (max 0 (min 255 %))) channels))))

(defn hex->rgb [hex]
  (let [digits (str/replace hex "#" "")]
    (mapv #(number/parse-hex (subs digits % (+ % 2))) [0 2 4])))

(defn rgb [r g b]
  {:kind :rgb :hex (rgb-array->hex [r g b])})

(def gradient {:kind :gradient})

(defn grey []
  (rgb 147 153 178))

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
    (interpolate-stops [166 227 161] [249 226 175] [243 139 168] percentage 50)))

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
    "linear-gradient(90deg, #a6e3a1, #f9e2af, #f38ba8)"
    (css colour 50)))
