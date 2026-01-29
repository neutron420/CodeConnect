"use client";
import React from "react";
import { motion } from "framer-motion";
import Image from "next/image";

export const TestimonialsColumn = (props: {
  className?: string;
  testimonials: Array<{
    text: string;
    image: string;
    name: string;
    role: string;
  }>;
  duration?: number;
}) => {
  return (
    <div className={props.className}>
      <motion.div
        animate={{
          translateY: "-50%",
        }}
        transition={{
          duration: props.duration || 10,
          repeat: Infinity,
          ease: "linear",
          repeatType: "loop",
        }}
        className="flex flex-col gap-4 pb-4"
      >
        {[...Array(2)].map((_, index) => (
          <React.Fragment key={index}>
            {props.testimonials.map(({ text, image, name, role }, i) => (
              <div
                // CHANGED: Added white background and dark text colors
                className="bg-white text-slate-700 p-10 rounded-3xl border border-slate-200 shadow-lg shadow-primary/10 max-w-xs w-full"
                key={`${index}-${i}`}
              >
                <div>{text}</div>
                <div className="flex items-center gap-2 mt-5">
                  <Image
                    src={image}
                    alt={name}
                    width={40}
                    height={40}
                    className="h-10 w-10 rounded-full"
                  />
                  <div className="flex flex-col">
                    <div className="font-medium tracking-tight leading-5 text-slate-900">{name}</div>
                    <div className="leading-5 tracking-tight text-slate-500">{role}</div>
                  </div>
                </div>
              </div>
            ))}
          </React.Fragment>
        ))}
      </motion.div>
    </div>
  );
};