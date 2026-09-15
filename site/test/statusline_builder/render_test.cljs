(ns statusline-builder.render-test
  (:require [clojure.test :refer [deftest is]]
            [statusline-builder.dom :as dom]
            [statusline-builder.render :as render]))

(deftest animation-identities-distinguish-repeated-pieces
  (let [module {:id "LlmInsight" :repeatable true}
        keys (mapv #(render/animation-key module %) [nil "first" "second"])]
    (is (= 3 (count (set keys)))))
  (let [module {:id "Model" :repeatable false}]
    (is (= (render/animation-key module nil)
           (render/animation-key module "model-instance")))))

(deftest each-repeated-piece-animates-from-its-own-position
  (let [module {:id "LlmInsight" :repeatable true}
        offsets (atom {})
        positions (atom {"first" 10 "second" 50})
        node (fn [id]
               #js {:getAttribute (fn [attribute]
                                    (when (= attribute "data-animation-key")
                                      (render/animation-key module id)))
                    :getBoundingClientRect (fn [] #js {:left (get @positions id) :top 0})
                    :animate (fn [frames _]
                               (swap! offsets assoc id (.-transform (aget frames 0))))})
        nodes [(node "first") (node "second")]]
    (with-redefs [dom/nodes (constantly nodes)
                  dom/reduced-motion? (constantly false)]
      (let [before (render/capture-positions)]
        (reset! positions {"first" 30 "second" 80})
        (render/play-flip! before)
        (is (= {"first" "translate(-20px, 0px)"
                "second" "translate(-30px, 0px)"}
               @offsets))))))
