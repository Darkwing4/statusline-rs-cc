(ns statusline-builder.every-segment-ron
  (:require [statusline-builder.catalogue-file :as catalogue-file]
            [statusline-builder.editor-state :as es]
            [statusline-builder.ron :as ron]
            [statusline-builder.segment :as segment]))

(defn -main [& _]
  (let [installed (es/install-catalogue es/initial (catalogue-file/read-catalogue))
        text (ron/generate (assoc installed :segments (mapv segment/create (:modules installed))))]
    #?(:cljs (.write js/process.stdout text)
       :clj (do (print text) (flush)))))
