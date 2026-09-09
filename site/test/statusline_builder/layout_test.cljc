(ns statusline-builder.layout-test
  (:require [clojure.test :refer [deftest is]]
            [statusline-builder.layout :as layout]
            [statusline-builder.width :as width]))

(defn- pieces [text]
  [{:text text :colour "#ffffff"}])

(defn- entry
  ([text] (entry text false))
  ([text standalone] {:pieces (pieces text) :standalone standalone}))

(defn- texts [row]
  (map #(:text (first (:pieces (:entry %)))) (:cells row)))

(deftest standalone-pieces-break-the-line-where-they-sit
  (let [rows (layout/layout-rows [(entry "one") (entry "below" true) (entry "two") (entry "three") (entry "also below" true)] " " 80)]
    (is (= [false true false true] (map :standalone rows)))
    (is (= ["one"] (texts (nth rows 0))))
    (is (= ["below"] (texts (nth rows 1))))
    (is (= ["two" "three"] (texts (nth rows 2))))
    (is (= 0 (count (:cells (first (layout/layout-rows [(entry "below" true)] " " 80))))))
    (is (= 3 (count (layout/layout-rows [(entry "below" true) (entry "after")] " " 80))))
    (is (= 3 (count (layout/layout-rows [(entry "one") (entry "below" true) (entry "also" true)] " " 80))))))

(deftest dragged-columns-snap-to-the-slider-steps
  (is (= 148 (layout/snap-columns 147.2)))
  (is (= 152 (layout/snap-columns 150)))
  (is (= 40 (layout/snap-columns -30)))
  (is (= 320 (layout/snap-columns 900))))

(deftest preview-wrapping-uses-cols-minus-four-and-never-leads-with-a-separator
  (let [separators (fn [columns] (map #(map :separator %) (layout/wrap-preview-segments [(entry "one") (entry "two")] " | " (layout/runtime-preview-columns columns))))]
    (is (= 8 (layout/runtime-preview-columns 12)))
    (is (= [[false] [false]] (separators 12)))
    (is (= [[false true]] (separators 13)))))

(deftest line-fill-reports-the-widest-row-and-the-row-count
  (is (= "0 of 92 cols, 0 rows" (layout/describe-line-fill [] " " 92)))
  (is (= "9 of 92 cols, 1 row" (layout/describe-line-fill [(entry "one") (entry "two")] " | " 92)))
  (is (= "3 of 8 cols, 2 rows" (layout/describe-line-fill [(entry "one") (entry "two")] " | " 8)))
  (is (= "10 of 92 cols, 2 rows" (layout/describe-line-fill [(entry "one") (entry "standalone" true)] " " 92))))

(deftest a-drop-lands-on-the-nearest-slot-of-the-row-under-the-cursor
  (let [box (fn [id left right top bottom] {:node id :rect {:left left :right right :top top :bottom bottom}})
        boxes [(box "a" 0 40 0 20) (box "b" 50 90 0 20) (box "c" 0 40 30 50)]]
    (is (= {:node "b" :before true} (layout/nearest-drop-slot boxes 48 10)))
    (is (= {:node "c" :before false} (layout/nearest-drop-slot boxes 45 27)))
    (is (= {:node "b" :before false} (layout/nearest-drop-slot boxes 200 10)))
    (is (= {:node "c" :before false} (layout/nearest-drop-slot boxes 200 200)))
    (is (nil? (layout/nearest-drop-slot [] 10 10)))))

(deftest ascii-and-wide-code-points-count-terminal-columns
  (is (= 5 (width/display-width "a界bc")))
  (is (= 0 (width/display-width "\u2060")))
  (is (= 1 (width/display-width "é"))))

#?(:cljs
   (deftest preview-width-handles-terminal-graphemes
     (is (= 5 (width/display-width "a界🙂")))
     (is (= 2 (width/display-width "👩‍💻")))
     (is (= 2 (width/display-width "🇺🇸")))
     (is (= 2 (width/display-width "1️⃣")))))

#?(:cljs
   (deftest flag-and-keycap-widths-preserve-wrapping-boundaries
     (is (= 1 (count (layout/wrap-preview-segments [(entry "a") (entry "🇺🇸")] " " 4))))
     (is (= 2 (count (layout/wrap-preview-segments [(entry "a") (entry "🇺🇸")] " " 3))))
     (is (= 1 (count (layout/wrap-preview-segments [(entry "a") (entry "1️⃣")] " " 4))))
     (is (= 2 (count (layout/wrap-preview-segments [(entry "a") (entry "1️⃣")] " " 3))))))

(deftest a-line-break-ends-the-row-and-stays-visible-at-its-end
  (let [break {:pieces (pieces "\u2060") :standalone false :line-break true}
        rows (layout/layout-rows [break (entry "one") break break (entry "two") (entry "three")] " " 80)]
    (is (= [false false] (map :standalone rows)))
    (is (= ["\u2060" "one" "\u2060"] (texts (nth rows 0))))
    (is (= ["\u2060" "two" "three"] (texts (nth rows 1))))
    (is (= "9 of 80 cols, 2 rows" (layout/describe-line-fill [(entry "one") break (entry "two") (entry "three")] " " 80)))))

(deftest standalone-rows-follow-the-terminal-width
  (let [long (entry "one two three" true)
        first-text (fn [rows] (first (texts (second rows))))]
    (is (= "one two three" (first-text (layout/layout-rows [long] " " 13))))
    (is (= "one two\nthree" (first-text (layout/layout-rows [long] " " 7))))
    (is (= "one tw…" (first-text (layout/layout-rows [(assoc long :overflow :truncate)] " " 7))))
    (is (= "7 of 7 cols, 2 rows" (layout/describe-line-fill [long] " " 7)))))

(deftest wrapping-and-truncating-keep-the-piece-colours
  (let [pieces [{:text "💡 goal:" :colour "a"} {:text " ship it now" :colour "b"}]]
    (is (= [{:text "💡 goal:\n" :colour "a"} {:text "ship it now" :colour "b"}] (layout/wrap-pieces pieces 12)))
    (is (= [{:text "💡 goal:" :colour "a"} {:text " shi…" :colour "b"}] (layout/truncate-pieces pieces 13)))))
