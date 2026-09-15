(ns statusline-builder.controls
  (:require [statusline-builder.catalogue :as catalogue]
            [statusline-builder.colour :as colour]
            [statusline-builder.dom :refer [el]]
            [statusline-builder.shell-words :as shell-words]))

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

(defn text-control
  ([label-text value on-change max-length]
   (text-control label-text value on-change max-length false))
  ([label-text value on-change max-length wide?]
   (let [input (text-input value max-length)]
     (.addEventListener input "input" #(on-change (.-value input)))
     (labelled (if wide? "control-label control-wide" "control-label") label-text input))))

(defn- text-area-control [label-text value on-change max-length]
  (let [input (el "textarea" "control-input control-textarea")]
    (set! (.-value input) value)
    (set! (.-maxLength input) max-length)
    (set! (.-rows input) 3)
    (set! (.-spellcheck input) false)
    (.addEventListener input "input" #(on-change (.-value input)))
    (labelled "control-label control-wide" label-text input)))

(defn command-line-control [command args hint on-change]
  (doto (text-control "Command" (shell-words/join (into [command] args))
                      (fn [line]
                        (let [[command & args] (shell-words/split line)]
                          (on-change (or command "") (vec args))))
                      2000 true)
    (.append (el "span" "control-hint" hint))))

(defn- number-control [label-text {:keys [min max step]} value on-change]
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
                               bounded (-> (if (js/Number.isFinite typed) typed value)
                                           (clojure.core/max min)
                                           (clojure.core/min max))
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

(defn- without-row [rows index]
  (into (subvec rows 0 index) (subvec rows (inc index))))

(defn- remove-row-button [label-text index rows on-change]
  (button "list-remove" "×" (str "Remove " label-text " " (inc index))
          #(on-change (without-row @rows index))))

(defn- list-control [label-text value on-change]
  (let [items (atom (mapv str value))
        row (fn [index item]
              (let [input (text-input item 256)
                    node (el "div" "list-row")]
                (.setAttribute input "aria-label" (str label-text " " (inc index)))
                (.addEventListener input "input" #(on-change (swap! items assoc index (.-value input))))
                (.append node input (remove-row-button label-text index items on-change))
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
                (.append node (remove-row-button label-text index pairs on-change))
                node))]
    (rows-control label-text (map-indexed row @pairs) #(on-change (conj @pairs ["" ""])))))

(defn- tile-key [tile-colour]
  (if (= :rgb (:kind tile-colour)) (:hex tile-colour) "gradient"))

(defn- press-tile! [grid value]
  (let [pressed (tile-key value)]
    (doseq [tile (array-seq (.-children grid))]
      (.setAttribute tile "aria-pressed" (str (= pressed (.getAttribute tile "data-tile")))))))

(defn- palette-tile [grid label-text tile-name tile-colour on-pick]
  (let [tile (button "palette-tile" "" (str label-text " " tile-name)
                     (fn [] (press-tile! grid tile-colour) (on-pick tile-colour)))]
    (set! (.-title tile) tile-name)
    (.setAttribute tile "data-tile" (tile-key tile-colour))
    (.setProperty (.-style tile) "--swatch" (colour/swatch tile-colour))
    tile))

(defn- palette-grid [label-text value allow-gradient on-pick]
  (let [grid (el "div" "palette-grid")]
    (.setAttribute grid "role" "group")
    (.setAttribute grid "aria-label" (str label-text " palette"))
    (doseq [[tile-name hex] colour/palette]
      (.append grid (palette-tile grid label-text tile-name {:kind :rgb :hex hex} on-pick)))
    (when allow-gradient
      (.append grid (palette-tile grid label-text "Gradient" colour/gradient on-pick)))
    (press-tile! grid value)
    grid))

(defn colour-control [label-text value on-change allow-gradient]
  (let [wrapper (el "div" "color-control")
        heading (el "div" "color-control-label")
        swatch (el "span" "color-swatch")
        custom (el "label" "color-custom")
        custom-input (el "input" "rgb-input")
        pick! (fn [next]
                (.setProperty (.-style swatch) "--swatch" (colour/swatch next))
                (when (= :rgb (:kind next))
                  (set! (.-value custom-input) (:hex next)))
                (on-change next))
        grid (palette-grid label-text value allow-gradient pick!)]
    (.setAttribute swatch "aria-hidden" "true")
    (.setProperty (.-style swatch) "--swatch" (colour/swatch value))
    (.append heading (el "span" "" label-text) swatch)
    (set! (.-type custom-input) "color")
    (set! (.-value custom-input) (if (= :rgb (:kind value)) (:hex value) "#9399b2"))
    (.setAttribute custom-input "aria-label" (str label-text " custom color"))
    (.addEventListener custom-input "input"
                       (fn []
                         (let [next {:kind :rgb :hex (.-value custom-input)}]
                           (press-tile! grid next)
                           (pick! next))))
    (.append custom custom-input (el "span" "color-custom-label" "custom"))
    (.append wrapper heading grid custom)
    wrapper))

(defn control [field type value on-change]
  (let [label-text (catalogue/field-label (:name field))
        node (case (:kind field)
               "color" (colour-control label-text value on-change (contains? gradient-fields (str type "." (:name field))))
               "bool" (boolean-control label-text value on-change)
               "enum" (select-control label-text (:variants field) value on-change)
               "integer" (number-control label-text integer-range value on-change)
               "float" (number-control label-text (float-range (:name field)) value on-change)
               "list" (list-control label-text value on-change)
               "pairs" (pairs-control label-text value on-change)
               (if (= "prompt" (:name field))
                 (text-area-control label-text value on-change 2000)
                 (text-control label-text value on-change 256)))]
    (.append node (el "span" "control-hint" (:hint field)))
    node))
