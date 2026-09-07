(ns statusline-builder.controls
  (:require [statusline-builder.catalogue :as catalogue]
            [statusline-builder.colour :as colour]
            [statusline-builder.dom :refer [el]]))

(def ^:private gradient-fields #{"ContextUsage.color" "PromptCacheTtl.color"})

(def ^:private integer-range {:min 0 :max 1000000000 :step 1})

(defn- float-range [field-name]
  (if (= "gradient_midpoint_percentage" field-name)
    {:min 0.1 :max 99.9 :step 0.1}
    {:min 0 :max 1000000000 :step 0.1}))

(defn- labelled [class label-text input]
  (doto (el "label" class)
    (.append (el "span" "" label-text))
    (.append input)))

(defn- text-input [value max-length]
  (let [input (el "input" "control-input")]
    (set! (.-type input) "text")
    (set! (.-value input) value)
    (set! (.-maxLength input) max-length)
    (set! (.-autocomplete input) "off")
    (set! (.-spellcheck input) false)
    input))

(defn- button [class text aria-label on-click]
  (let [node (el "button" class text)]
    (set! (.-type node) "button")
    (.setAttribute node "aria-label" aria-label)
    (.addEventListener node "click" on-click)
    node))

(defn- option [text value]
  (let [node (el "option" "" text)]
    (set! (.-value node) value)
    node))

(defn text-control [label-text value on-change max-length]
  (let [input (text-input value max-length)]
    (.addEventListener input "input" #(on-change (.-value input)))
    (labelled "control-label" label-text input)))

(defn- number-control [label-text {:keys [min max step] :as range} value on-change]
  (let [input (el "input" "control-input")]
    (set! (.-type input) "number")
    (set! (.-value input) (str value))
    (set! (.-min input) (str min))
    (set! (.-max input) (str max))
    (set! (.-step input) (str step))
    (.addEventListener input "input"
                       (fn []
                         (when (and (seq (.-value input)) (.-valid (.-validity input)))
                           (let [number (js/Number (.-value input))]
                             (when (js/Number.isFinite number)
                               (on-change number))))))
    (.addEventListener input "change"
                       (fn []
                         (let [typed (js/Number (.-value input))
                               bounded (-> (if (js/Number.isFinite typed) typed value) (clojure.core/max min) (clojure.core/min max))
                               number (if (= 1 step) (js/Math.round bounded) bounded)]
                           (set! (.-value input) (str number))
                           (on-change number))))
    (labelled "control-label" label-text input)))

(defn- select-control [label-text variants value on-change]
  (let [select (el "select" "control-input select-control")]
    (doseq [variant variants]
      (.append select (option (catalogue/segment-label variant) variant)))
    (set! (.-value select) value)
    (.addEventListener select "change" #(on-change (.-value select)))
    (labelled "control-label" label-text select)))

(defn- boolean-control [label-text value on-change]
  (let [input (el "input")]
    (set! (.-type input) "checkbox")
    (set! (.-checked input) (boolean value))
    (.addEventListener input "change" #(on-change (.-checked input)))
    (labelled "toggle-control" label-text input)))

(defn- rows-control [label-text rows add-row]
  (let [wrapper (el "div" "list-control")
        container (el "div" "list-rows")]
    (.append wrapper (el "span" "list-control-label" label-text))
    (doseq [row rows]
      (.append container row))
    (.append wrapper container (button "list-add" "+ add" (str "Add " label-text) add-row))
    wrapper))

(defn- list-control [label-text value on-change]
  (let [items (atom (mapv str value))
        row (fn [index item]
              (let [input (text-input item 256)
                    node (el "div" "list-row")]
                (.setAttribute input "aria-label" (str label-text " " (inc index)))
                (.addEventListener input "input" #(on-change (swap! items assoc index (.-value input))))
                (.append node input (button "list-remove" "×" (str "Remove " label-text " " (inc index))
                                           #(on-change (vec (keep-indexed (fn [at entry] (when (not= at index) entry)) @items)))))
                node))]
    (rows-control label-text (map-indexed row @items) #(on-change (conj @items "")))))

(defn- pairs-control [label-text value on-change]
  (let [pairs (atom (mapv (fn [[from to]] [(str from) (str to)]) value))
        row (fn [index pair]
              (let [node (el "div" "list-row list-row-pair")]
                (doseq [[position side] [[0 "from"] [1 "to"]]]
                  (let [input (text-input (pair position) 256)]
                    (set! (.-placeholder input) side)
                    (.setAttribute input "aria-label" (str label-text " " (inc index) " " side))
                    (.addEventListener input "input" #(on-change (swap! pairs assoc-in [index position] (.-value input))))
                    (.append node input)))
                (.append node (button "list-remove" "×" (str "Remove " label-text " " (inc index))
                                      #(on-change (vec (keep-indexed (fn [at entry] (when (not= at index) entry)) @pairs)))))
                node))]
    (rows-control label-text (map-indexed row @pairs) #(on-change (conj @pairs ["" ""])))))

(def ^:private colour-kinds [[:named "ANSI"] [:rgb "RGB"] [:gradient "Gradient"]])

(defn- kind-select [label-text value allow-gradient on-change]
  (let [select (el "select" "color-kind")]
    (doseq [[kind name] (if allow-gradient colour-kinds (butlast colour-kinds))]
      (.append select (option name (clojure.core/name kind))))
    (set! (.-value select) (name (:kind value)))
    (.setAttribute select "aria-label" (str label-text " color type"))
    (.addEventListener select "change"
                       (fn []
                         (on-change (case (.-value select)
                                      "named" (colour/grey)
                                      "rgb" (apply colour/rgb (colour/colour->rgb value [183 165 255]))
                                      colour/gradient))))
    select))

(defn- colour-editor [label-text value swatch on-change]
  (let [paint! (fn [next] (.setProperty (.-style swatch) "--swatch" (colour/swatch next)) (on-change next))]
    (case (:kind value)
      :named (let [select (el "select" "ansi-select")]
               (.setAttribute select "aria-label" (str label-text " ANSI color"))
               (doseq [[code name] colour/ansi]
                 (.append select (option (str code " · " name) (str code))))
               (set! (.-value select) (str (:code value)))
               (.addEventListener select "change" #(paint! (colour/named (js/Number (.-value select)))))
               select)
      :rgb (let [input (el "input" "rgb-input")]
             (set! (.-type input) "color")
             (set! (.-value input) (:hex value))
             (.setAttribute input "aria-label" (str label-text " RGB color"))
             (.addEventListener input "input" #(paint! {:kind :rgb :hex (.-value input)}))
             input)
      (el "div" "gradient-readout" "follows the value"))))

(defn colour-control [label-text value on-change allow-gradient]
  (let [wrapper (el "div" "color-control")
        heading (el "div" "color-control-label")
        swatch (el "span" "color-swatch")
        editor (el "div" "color-editor")]
    (.setAttribute swatch "aria-hidden" "true")
    (.setProperty (.-style swatch) "--swatch" (colour/swatch value))
    (.append heading (el "span" "" label-text) swatch)
    (.append editor (kind-select label-text value allow-gradient on-change) (colour-editor label-text value swatch on-change))
    (.append wrapper heading editor)
    wrapper))

(defn control [field type value on-change]
  (let [label-text (catalogue/field-label (:name field))]
    (case (:kind field)
      "color" (colour-control label-text value on-change (contains? gradient-fields (str type "." (:name field))))
      "bool" (boolean-control label-text value on-change)
      "enum" (select-control label-text (:variants field) value on-change)
      "integer" (number-control label-text integer-range value on-change)
      "float" (number-control label-text (float-range (:name field)) value on-change)
      "list" (list-control label-text value on-change)
      "pairs" (pairs-control label-text value on-change)
      (text-control label-text value on-change (if (= "prompt" (:name field)) 2000 256)))))
