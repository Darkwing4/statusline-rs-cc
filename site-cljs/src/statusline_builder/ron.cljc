(ns statusline-builder.ron
  (:require [clojure.string :as str]
            [statusline-builder.catalogue :as catalogue]
            [statusline-builder.colour :as colour]
            [statusline-builder.editor-state :as es]
            [statusline-builder.number :as number]))

(defn- ron-string [value]
  (str "\""
       (str/escape (str value) {\\ "\\\\" \" "\\\"" \backspace "\\b" \formfeed "\\f" \newline "\\n" \return "\\r" \tab "\\t"})
       "\""))

(defn- ron-float [value]
  (if (number/integer-valued? value)
    (number/fixed1 value)
    (str value)))

(defn- colour->ron [colour]
  (case (:kind colour)
    :named (str "Named(" (:code colour) ")")
    :rgb (let [[r g b] (colour/hex->rgb (:hex colour))] (str "Rgb(" r ", " g ", " b ")"))
    "Gradient"))

(defn- value->ron [field value]
  (case (:kind field)
    "color" (colour->ron value)
    "text" (ron-string value)
    "bool" (if value "true" "false")
    "integer" (str (max 0 (number/round (or value 0))))
    "float" (ron-float value)
    "enum" value
    "list" (str "[" (str/join ", " (map ron-string value)) "]")
    "pairs" (str "[" (str/join ", " (map (fn [[from to]] (str "(" (ron-string from) ", " (ron-string to) ")")) value)) "]")
    (throw (ex-info (str "Cannot serialize field kind: " (:kind field)) {}))))

(defn segment->ron [state segment]
  (let [module (es/segment-module state segment)]
    (-> [(str "        " (:type segment) "(")]
        (into (map (fn [field] (str "            " (:name field) ": " (value->ron field (get-in segment [:config (:name field)])) ","))
                   (:fields module)))
        (conj "        ),"))))

(defn generate [state]
  (str (str/join "\n" (concat ["("
                               (str "    separator: " (ron-string (:separator state)) ",")
                               (str "    separator_color: " (colour->ron (:separator-color state)) ",")
                               "    segments: ["]
                              (mapcat #(segment->ron state %) (:segments state))
                              ["    ]," ")"]))
       "\n"))

(defn config-problems [state]
  (vec (for [segment (:segments state)
             :let [module (es/segment-module state segment)]
             field (:fields module)
             :when (and (= "pairs" (:kind field))
                        (some (fn [[from _]] (= "" from)) (get-in segment [:config (:name field)])))]
         (str (:label module) ": every " (str/lower-case (catalogue/field-label (:name field))) " row needs a left side."))))
